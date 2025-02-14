use std::{
    io::{Read, Write},
    sync::Arc,
};

use anyhow::{bail, Result};
use bitnuc::encode;
use cyto::{
    mappers::{MapperOffset, MappingError, ProbeMapper},
    statistics::{LibraryCombination, Statistics},
    GeometryR1, Mapper, MappingStatistics,
};
use paraseq::{
    fastq::Reader,
    fastx::Record,
    parallel::{PairedParallelProcessor, PairedParallelReader},
};
use parking_lot::Mutex;

use crate::io::open_file_handle;

#[derive(Clone)]
pub struct MappingProbeImplementor<M: Mapper> {
    target_mapper: M,
    probe_mapper: Arc<ProbeMapper>,
    target_offset: Option<MapperOffset>,
    probe_offset: Option<MapperOffset>,
    geometry: GeometryR1,

    local_stats: MappingStatistics,
    global_stats: Arc<Mutex<MappingStatistics>>,

    // Buffers for barcode and UMI sequences used during splitting
    barcode_buf: Vec<u64>,
    umi_buf: Vec<u64>,

    // Temporary buffer for output
    local_output_buffers: Vec<Vec<u8>>,

    // Output files (vector of file handles)
    files: Arc<Vec<Mutex<Box<dyn Write + Send>>>>,

    // Exact matching flag
    exact_matching: bool,
}
impl<M: Mapper> MappingProbeImplementor<M> {
    pub fn new(
        target_mapper: M,
        probe_mapper: Arc<ProbeMapper>,
        target_offset: Option<MapperOffset>,
        probe_offset: Option<MapperOffset>,
        geometry: GeometryR1,
        files: Arc<Vec<Mutex<Box<dyn Write + Send>>>>,
        exact_matching: bool,
    ) -> Self {
        let local_stats = MappingStatistics::default();
        let global_stats = Arc::new(Mutex::new(MappingStatistics::default()));

        let local_output_buffers = vec![Vec::new(); files.len()];

        Self {
            target_mapper,
            probe_mapper,
            target_offset,
            probe_offset,
            geometry,
            local_stats,
            global_stats,
            barcode_buf: Vec::new(),
            umi_buf: Vec::new(),
            local_output_buffers,
            files,
            exact_matching,
        }
    }

    fn encode_r1<R: Record>(&mut self, record: &R) -> Result<bool> {
        // clear encoding buffers
        self.barcode_buf.clear();
        self.umi_buf.clear();

        // Split R1 into barcode and UMI
        let seq = record.seq();
        if seq.len() != self.geometry.barcode + self.geometry.umi {
            bail!("R1 sequence length does not match provided geometry");
        }
        let seq_bc = &seq[..self.geometry.barcode];
        let seq_umi = &seq[self.geometry.barcode..];

        // encode barcode and UMI
        if encode(seq_bc, &mut self.barcode_buf).is_err()
            || encode(seq_umi, &mut self.umi_buf).is_err()
        {
            // If an error occurs, it is due to an `N` in either the barcode or UMI
            return Ok(false);
        }

        encode(seq_bc, &mut self.barcode_buf)?;
        encode(seq_umi, &mut self.umi_buf)?;

        if self.barcode_buf.len() != 1 || self.umi_buf.len() != 1 {
            bail!(
                "Barcode split assertion length failed - both barcode and UMI must be under 32bp"
            );
        }
        Ok(true)
    }

    fn statistics(&self) -> Statistics {
        Statistics::new(
            // LibraryCombination::Single(self.target_mapper.library_statistics()),
            LibraryCombination::Dual(
                self.target_mapper.library_statistics(),
                self.probe_mapper.library_statistics(),
            ),
            self.global_stats.lock().to_owned(),
        )
    }

    fn map_target(&self, seq: &[u8]) -> Result<usize, MappingError> {
        if self.exact_matching {
            self.target_mapper.map(seq, self.target_offset)
        } else {
            self.target_mapper.map_corrected(seq, self.target_offset)
        }
    }

    fn map_probe(&self, seq: &[u8]) -> Result<usize, MappingError> {
        if self.exact_matching {
            self.probe_mapper.map(seq, self.probe_offset)
        } else {
            self.probe_mapper.map_corrected(seq, self.probe_offset)
        }
    }
}
impl<M: Mapper> PairedParallelProcessor for MappingProbeImplementor<M> {
    // fn process_record_pair(&mut self, pair: binseq::RefRecordPair) -> Result<()> {
    fn process_record_pair<Rf: paraseq::fastx::Record>(
        &mut self,
        r1: Rf,
        r2: Rf,
    ) -> paraseq::parallel::Result<()> {
        // Split R1 into barcode and UMI and 2-bit encode them
        if !self.encode_r1(&r1)? {
            return Ok(()); // Skip the record if it contains an `N`
        }

        let barcode = self.barcode_buf[0];
        let umi = self.umi_buf[0];

        // Map the sequence
        match (self.map_target(r2.seq()), self.map_probe(r2.seq())) {
            (Ok(t_idx), Ok(p_idx)) => {
                // Create the record
                let record = ibu::Record::new(barcode, umi, t_idx as u64);

                // Identify the correct output buffer index
                let probe_alias_index = self
                    .probe_mapper
                    .get_alias_index(p_idx)
                    .expect("Could not access probe alias index");

                // Write the record to the correct output buffer
                record.write_bytes(&mut self.local_output_buffers[probe_alias_index])?;
            }
            (Err(why), Ok(_)) | (Ok(_), Err(why)) => {
                self.local_stats.increment_unmapped(why);
            }
            (Err(why1), Err(why2)) => {
                self.local_stats.increment_unmapped_multi_reason(why1, why2);
            }
        }

        Ok(())
    }

    fn on_batch_complete(&mut self) -> paraseq::parallel::Result<()> {
        // Write all local output buffers to the corresponding files
        for idx in 0..self.local_output_buffers.len() {
            // Scope the lock to ensure it is released early
            {
                let writer = &mut self.files[idx].lock();
                writer.write_all(&self.local_output_buffers[idx])?;
                writer.flush()?;
            }
            self.local_output_buffers[idx].clear();
        }

        self.global_stats.lock().merge(&self.local_stats);
        self.local_stats.clear();

        Ok(())
    }
}

pub fn ibu_map_probed_pairs_paraseq<M, R>(
    rdr_r1: Reader<R>,
    rdr_r2: Reader<R>,
    filenames: &[String],
    target_mapper: M,
    probe_mapper: ProbeMapper,
    target_offset: Option<MapperOffset>,
    probe_offset: Option<MapperOffset>,
    geometry: GeometryR1,
    num_threads: usize,
    exact_matching: bool,
) -> Result<Statistics>
where
    M: Mapper + 'static,
    R: Read + Send,
{
    // Initialize the header and write it to the output file
    let header = ibu::Header::try_from(geometry)?;

    // Open the output files
    let mut writers = filenames
        .iter()
        .map(|filename| open_file_handle(filename))
        .collect::<Result<Vec<_>>>()?;

    // Write the header to each output file
    for writer in writers.iter_mut() {
        header.write_bytes(writer)?;
        writer.flush()?;
    }

    // Wrap the writers vec to indepentently Mutex each writer
    let writers = Arc::new(writers.into_iter().map(Mutex::new).collect::<Vec<_>>());

    // Initialize the mapping implementor
    let implementor = MappingProbeImplementor::new(
        target_mapper,
        Arc::new(probe_mapper),
        target_offset,
        probe_offset,
        geometry,
        writers,
        exact_matching,
    );

    // Process the records in parallel
    rdr_r1.process_parallel_paired(rdr_r2, implementor.clone(), num_threads)?;

    // Return the statistics
    Ok(implementor.statistics())
}
