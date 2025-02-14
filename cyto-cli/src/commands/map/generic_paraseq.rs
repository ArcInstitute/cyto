use std::{
    io::{Read, Write},
    sync::Arc,
};

use anyhow::{bail, Result};
use bitnuc::encode;
use cyto::{
    mappers::{MapperOffset, MappingError},
    statistics::{LibraryCombination, Statistics},
    GeometryR1, Mapper, MappingStatistics,
};
use paraseq::{
    fastq::Reader,
    fastx::Record,
    parallel::{PairedParallelProcessor, PairedParallelReader},
};
use parking_lot::Mutex;

use super::utils::{open_handle, reopen_handle};

#[derive(Debug, Clone)]
pub struct MappingImplementor<M: Mapper> {
    target_mapper: M,
    target_offset: Option<MapperOffset>,
    geometry: GeometryR1,

    local_stats: MappingStatistics,
    global_stats: Arc<Mutex<MappingStatistics>>,

    // Buffers for barcode and UMI sequences used during splitting
    barcode_buf: Vec<u64>,
    umi_buf: Vec<u64>,

    // Temporary buffer for output
    local_output_buf: Vec<u8>,

    // Output file name
    filename: String,

    // Lock used to synchronize access to output writer
    output_lock: Arc<Mutex<()>>,

    // Exact matching flag
    exact_matching: bool,

    // thread id
    tid: usize,
}
impl<M: Mapper> MappingImplementor<M> {
    pub fn new(
        target_mapper: M,
        target_offset: Option<MapperOffset>,
        geometry: GeometryR1,
        filename: String,
        exact_matching: bool,
    ) -> Self {
        let local_stats = MappingStatistics::default();
        let global_stats = Arc::new(Mutex::new(MappingStatistics::default()));
        Self {
            target_mapper,
            target_offset,
            geometry,
            local_stats,
            global_stats,
            barcode_buf: Vec::new(),
            umi_buf: Vec::new(),
            local_output_buf: Vec::new(),
            filename,
            output_lock: Arc::new(Mutex::new(())),
            exact_matching,
            tid: 0,
        }
    }

    fn split_r1<R: Record>(&mut self, record: &R) -> Result<bool> {
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
            LibraryCombination::Single(self.target_mapper.library_statistics()),
            self.global_stats.lock().to_owned(),
        )
    }

    fn map_sequence(&self, seq: &[u8]) -> Result<usize, MappingError> {
        if self.exact_matching {
            self.target_mapper.map(seq, self.target_offset)
        } else {
            self.target_mapper.map_corrected(seq, self.target_offset)
        }
    }
}
impl<M: Mapper> PairedParallelProcessor for MappingImplementor<M> {
    // fn process_record_pair(&mut self, pair: binseq::RefRecordPair) -> Result<()> {
    fn process_record_pair<Rf: paraseq::fastx::Record>(
        &mut self,
        r1: Rf,
        r2: Rf,
    ) -> paraseq::parallel::Result<()> {
        // Split R1 into barcode and UMI
        if !self.split_r1(&r1)? {
            return Ok(()); // Skip the record if it contains an `N`
        }

        let barcode = self.barcode_buf[0];
        let umi = self.umi_buf[0];

        // Map the sequence
        match self.map_sequence(r2.seq()) {
            Ok(index) => {
                // Write the record
                let record = ibu::Record::new(barcode, umi, index as u64);
                record.write_bytes(&mut self.local_output_buf)?;
                self.local_stats.increment_mapped();
            }
            Err(why) => {
                self.local_stats.increment_unmapped(why);
            }
        }

        Ok(())
    }

    fn on_batch_complete(&mut self) -> paraseq::parallel::Result<()> {
        // Scope the lock to ensure it is released
        {
            let _lock = self.output_lock.lock();
            let mut writer = reopen_handle(&self.filename)?;
            writer.write_all(&self.local_output_buf)?;
            self.local_output_buf.clear();
            writer.flush()?;
        }

        self.global_stats.lock().merge(&self.local_stats);
        self.local_stats.clear();

        Ok(())
    }

    fn set_thread_id(&mut self, thread_id: usize) {
        self.tid = thread_id;
    }

    fn get_thread_id(&self) -> usize {
        self.tid
    }
}

pub fn ibu_map_pairs_paraseq<M, R>(
    rdr_r1: Reader<R>,
    rdr_r2: Reader<R>,
    filename: String,
    target_mapper: M,
    target_offset: Option<MapperOffset>,
    geometry: GeometryR1,
    num_threads: usize,
    exact_matching: bool,
) -> Result<Statistics>
where
    M: Mapper + 'static,
    R: Read + Send,
{
    // Initialize the header and write it to the output file
    {
        let header = ibu::Header::try_from(geometry)?;
        let mut writer = open_handle(&filename)?;
        header.write_bytes(&mut writer)?;
        writer.flush()?;
    }

    // Initialize the mapping implementor
    let implementor = MappingImplementor::new(
        target_mapper,
        target_offset,
        geometry,
        filename,
        exact_matching,
    );

    // Process the records in parallel
    rdr_r1.process_parallel_paired(rdr_r2, implementor.clone(), num_threads)?;

    // Return the statistics
    Ok(implementor.statistics())
}
