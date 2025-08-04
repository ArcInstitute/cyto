use std::{
    io::{Read, Write},
    sync::Arc,
    time::Instant,
};

use anyhow::{Result, bail};
use binseq::prelude::*;
use bitnuc::encode;
use cyto_core::{
    GeometryR1, Mapper, MappingStatistics,
    mappers::{Adjustment, MapperOffset, MappingError, ProbeMapper},
    statistics::{LibraryCombination, RuntimeStatistics, Statistics},
};
use cyto_io::open_file_handle;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use log::info;
use paraseq::prelude::*;
use parking_lot::Mutex;

#[derive(Clone)]
pub struct MappingProbeImplementor<M: Mapper> {
    target_mapper: Arc<M>,
    probe_mapper: Arc<ProbeMapper>,
    target_offset: Option<MapperOffset>,
    probe_offset: Option<MapperOffset>,
    geometry: GeometryR1,
    adjustment: Option<Adjustment>,

    local_stats: MappingStatistics,
    global_stats: Arc<Mutex<MappingStatistics>>,

    // Buffers for barcode and UMI sequences used during splitting
    barcode_buf: Vec<u64>,
    umi_buf: Vec<u64>,

    // Temporary decoding buffer for R2 sequences (binseq)
    dbuf: Vec<u8>,

    // Temporary buffer for output
    local_output_buffers: Vec<Vec<u8>>,

    // Output files (vector of file handles)
    files: Arc<Vec<Mutex<Box<dyn Write + Send>>>>,

    // Exact matching flag
    exact_matching: bool,

    // thread id
    tid: usize,

    // progress bar
    pbar: Arc<Mutex<ProgressBar>>,

    // init time
    init_time: Instant,

    // mapping start time
    map_time: Instant,
}
impl<M: Mapper> MappingProbeImplementor<M> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        target_mapper: Arc<M>,
        probe_mapper: Arc<ProbeMapper>,
        target_offset: Option<MapperOffset>,
        probe_offset: Option<MapperOffset>,
        geometry: GeometryR1,
        files: Arc<Vec<Mutex<Box<dyn Write + Send>>>>,
        exact_matching: bool,
        adjustment: bool,
        init_time: Instant,
    ) -> Self {
        let local_stats = MappingStatistics::default();
        let global_stats = Arc::new(Mutex::new(MappingStatistics::default()));
        let adjustment = if adjustment {
            Some(Adjustment::default())
        } else {
            None
        };

        let local_output_buffers = vec![Vec::new(); files.len()];
        let pbar = ProgressBar::new_spinner();
        pbar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} [{elapsed_precise}] {msg}")
                .unwrap(),
        );
        pbar.set_draw_target(ProgressDrawTarget::stderr_with_hz(20));

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
            dbuf: Vec::new(),
            local_output_buffers,
            files,
            exact_matching,
            tid: 0,
            pbar: Arc::new(Mutex::new(pbar)),
            init_time,
            map_time: Instant::now(),
            adjustment,
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

        if self.barcode_buf.len() != 1 || self.umi_buf.len() != 1 {
            bail!(
                "Barcode split assertion length failed - both barcode and UMI must be under 32bp"
            );
        }
        Ok(true)
    }

    fn split_r1<B: BinseqRecord>(&mut self, record: &B) -> Result<()> {
        // Split R1 into barcode and UMI
        if record.slen() as usize != self.geometry.barcode + self.geometry.umi {
            bail!("R1 sequence length does not match provided geometry");
        }
        bitnuc::split_packed(
            record.sbuf(),
            record.slen() as usize,
            self.geometry.barcode,
            &mut self.barcode_buf,
            &mut self.umi_buf,
        )?;
        if self.barcode_buf.len() != 1 || self.umi_buf.len() != 1 {
            bail!(
                "Barcode split assertion length failed - both barcode and UMI must be under 32bp"
            );
        }
        Ok(())
    }

    fn decode_r2<B: BinseqRecord>(&mut self, record: &B) -> Result<()> {
        self.dbuf.clear();
        record.decode_x(&mut self.dbuf)?;
        Ok(())
    }

    fn statistics(&self) -> Statistics {
        let runtime = RuntimeStatistics::new(
            self.init_time.elapsed().as_secs_f64(),
            (self.map_time - self.init_time).as_secs_f64(),
            self.map_time.elapsed().as_secs_f64(),
            self.global_stats.lock().total_reads as f64 / self.map_time.elapsed().as_secs_f64(),
        );

        Statistics::new(
            // LibraryCombination::Single(self.target_mapper.library_statistics()),
            LibraryCombination::Dual(
                self.target_mapper.library_statistics(),
                self.probe_mapper.library_statistics(),
            ),
            self.global_stats.lock().to_owned(),
            runtime,
        )
    }

    fn map_target(&self, seq: &[u8]) -> Result<usize, MappingError> {
        if self.exact_matching {
            self.target_mapper
                .map(seq, self.target_offset, self.adjustment)
        } else {
            self.target_mapper
                .map_corrected(seq, self.target_offset, self.adjustment)
        }
    }

    fn map_probe(&self, seq: &[u8]) -> Result<usize, MappingError> {
        if self.exact_matching {
            self.probe_mapper
                .map(seq, self.probe_offset, Some(Adjustment::default()))
        } else {
            self.probe_mapper
                .map_corrected(seq, self.probe_offset, Some(Adjustment::default()))
        }
    }

    fn write_buffers(&mut self) -> Result<()> {
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
        Ok(())
    }

    fn update_stats(&mut self) {
        self.global_stats.lock().merge(&self.local_stats);
        self.local_stats.clear();
    }

    fn update_pbar(&self) {
        if self.tid == 0 {
            let (total, map_pc) = {
                let stats = self.global_stats.lock();
                (
                    stats.total_reads as f64 / 1_000_000.0,
                    stats.mapped_reads as f64 / stats.total_reads as f64 * 100.0,
                )
            };
            let elapsed = self.map_time.elapsed().as_secs_f64();
            let throughput = total / elapsed;
            // Lock the progress bar and update the message
            {
                let pb = self.pbar.lock();
                pb.set_message(format!(
                    "Processed: {total:.3}M reads ( Mapped: {map_pc:.2}%, Throughput: {throughput:.3}M/s )",
                ));
            }
        }
    }

    fn finish_pbar(&self) {
        let (total, map_pc) = {
            let stats = self.global_stats.lock();
            (
                stats.total_reads as f64 / 1_000_000.0,
                stats.mapped_reads as f64 / stats.total_reads as f64 * 100.0,
            )
        };
        let elapsed = self.map_time.elapsed().as_secs_f64();
        let throughput = total / elapsed;
        // Lock the progress bar and finish the message
        {
            let pb = self.pbar.lock();
            pb.finish_with_message(format!(
                "Mapping complete: {total:.3}M reads ( Mapped: {map_pc:.2}%, Throughput: {throughput:.3}M/s )",
            ));
        }
    }
}
impl<M: Mapper> PairedParallelProcessor for MappingProbeImplementor<M> {
    // fn process_record_pair(&mut self, pair: binseq::RefRecordPair) -> Result<()> {
    fn process_record_pair<Rf: paraseq::Record>(
        &mut self,
        r1: Rf,
        r2: Rf,
    ) -> paraseq::parallel::Result<()> {
        // Split R1 into barcode and UMI and 2-bit encode them
        if !self.encode_r1(&r1)? {
            return Ok(()); // Skip the record if it contains an `N`
        }

        // Shorthand the barcode and UMI
        let barcode = self.barcode_buf[0];
        let umi = self.umi_buf[0];

        // Map the sequence
        match (self.map_target(&r2.seq()), self.map_probe(&r2.seq())) {
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

                // Increment the mapped reads counter
                self.local_stats.increment_mapped();
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
        self.write_buffers()?;
        self.update_stats();
        self.update_pbar();
        Ok(())
    }

    fn set_thread_id(&mut self, thread_id: usize) {
        self.tid = thread_id;
    }
}

impl<M: Mapper> binseq::ParallelProcessor for MappingProbeImplementor<M> {
    // fn process_record_pair(&mut self, pair: binseq::RefRecordPair) -> Result<()> {
    fn process_record<B: BinseqRecord>(&mut self, pair: B) -> binseq::Result<()> {
        // Split R1 into barcode and UMI
        self.split_r1(&pair)?;

        // Decode R2
        self.decode_r2(&pair)?;

        // Shorthand the barcode and UMI and sequence
        let barcode = self.barcode_buf[0];
        let umi = self.umi_buf[0];
        let seq = &self.dbuf;

        // Map the sequence
        match (self.map_target(seq), self.map_probe(seq)) {
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

                // Increment the mapped reads counter
                self.local_stats.increment_mapped();
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

    fn on_batch_complete(&mut self) -> Result<(), binseq::Error> {
        self.write_buffers()?;
        self.update_stats();
        self.update_pbar();
        Ok(())
    }

    fn set_tid(&mut self, tid: usize) {
        self.tid = tid;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn ibu_map_probed_pairs_paraseq<M, R>(
    rdr_r1: paraseq::fastx::Reader<R>,
    rdr_r2: paraseq::fastx::Reader<R>,
    filenames: &[String],
    target_mapper: Arc<M>,
    probe_mapper: Arc<ProbeMapper>,
    target_offset: Option<MapperOffset>,
    probe_offset: Option<MapperOffset>,
    geometry: GeometryR1,
    num_threads: usize,
    exact_matching: bool,
    adjustment: bool,
    start_time: Instant,
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
    for writer in &mut writers {
        header.write_bytes(writer)?;
        writer.flush()?;
    }

    // Wrap the writers vec to indepentently Mutex each writer
    let writers = Arc::new(writers.into_iter().map(Mutex::new).collect::<Vec<_>>());

    // Initialize the mapping implementor
    let implementor = MappingProbeImplementor::new(
        target_mapper,
        probe_mapper,
        target_offset,
        probe_offset,
        geometry,
        writers,
        exact_matching,
        adjustment,
        start_time,
    );

    // Process the records in parallel
    info!("Beginning mapping with {num_threads} threads");
    rdr_r1.process_parallel_paired(rdr_r2, implementor.clone(), num_threads)?;

    // Finish the progress bar
    implementor.finish_pbar();

    // Return the statistics
    Ok(implementor.statistics())
}

#[allow(clippy::too_many_arguments)]
pub fn ibu_map_probed_pairs_binseq<M>(
    reader: BinseqReader,
    filenames: &[String],
    target_mapper: Arc<M>,
    probe_mapper: Arc<ProbeMapper>,
    target_offset: Option<MapperOffset>,
    probe_offset: Option<MapperOffset>,
    geometry: GeometryR1,
    num_threads: usize,
    exact_matching: bool,
    adjustment: bool,
    start_time: Instant,
) -> Result<Statistics>
where
    M: Mapper + 'static,
{
    // Initialize the header and write it to the output file
    let header = ibu::Header::try_from(geometry)?;

    // Open the output files
    let mut writers = filenames
        .iter()
        .map(|filename| open_file_handle(filename))
        .collect::<Result<Vec<_>>>()?;

    // Write the header to each output file
    for writer in &mut writers {
        header.write_bytes(writer)?;
        writer.flush()?;
    }

    // Wrap the writers
    let writers = Arc::new(writers.into_iter().map(Mutex::new).collect::<Vec<_>>());

    // Initialize the mapping implementor
    let implementor = MappingProbeImplementor::new(
        target_mapper,
        probe_mapper,
        target_offset,
        probe_offset,
        geometry,
        writers,
        exact_matching,
        adjustment,
        start_time,
    );

    // Process the records in parallel
    info!("Beginning mapping with {num_threads} threads");
    reader.process_parallel(implementor.clone(), num_threads)?;

    // Finish the progress bar
    implementor.finish_pbar();

    // Return the statistics
    Ok(implementor.statistics())
}
