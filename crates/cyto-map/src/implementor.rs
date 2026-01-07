use std::{
    io::{Read, Write},
    sync::Arc,
    time::Instant,
};

use anyhow::{Result, bail};
use binseq::{ParallelReader, prelude::*};
use bitnuc::twobit::encode_with_invalid;
use cyto_core::{
    GeometryR1, Mapper, MappingStatistics,
    mappers::{Adjustment, MapperOffset, MappingError},
    statistics::{LibraryCombination, RuntimeStatistics, Statistics},
};
use cyto_io::open_file_handle;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use log::info;
use paraseq::{
    fastx,
    prelude::{ParallelReader as ParaseqParallelReader, *},
};
use parking_lot::Mutex;

use super::{ILLUMINA_QUALITY_OFFSET, UMI_MIN_QUALITY};

#[derive(Clone)]
pub struct MappingImplementor<M: Mapper> {
    target_mapper: Arc<M>,
    target_offset: Option<MapperOffset>,
    geometry: GeometryR1,
    adjustment: Option<Adjustment>,
    umi_quality_removal: bool,

    local_stats: MappingStatistics,
    global_stats: Arc<Mutex<MappingStatistics>>,

    // Buffers for barcode and UMI sequences used during splitting
    barcode_buf: Vec<u64>,
    umi_buf: Vec<u64>,

    // Temporary buffer for output
    local_output_buf: Vec<u8>,

    // Output file handle
    file: Arc<Mutex<Box<dyn Write + Send>>>,

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
impl<M: Mapper> MappingImplementor<M> {
    pub fn new(
        target_mapper: Arc<M>,
        target_offset: Option<MapperOffset>,
        geometry: GeometryR1,
        file: Arc<Mutex<Box<dyn Write + Send>>>,
        exact_matching: bool,
        adjustment: bool,
        umi_quality_removal: bool,
        init_time: Instant,
    ) -> Self {
        let local_stats = MappingStatistics::default();
        let global_stats = Arc::new(Mutex::new(MappingStatistics::default()));
        let adjustment = if adjustment {
            Some(Adjustment::default())
        } else {
            None
        };

        let pbar = ProgressBar::new_spinner();
        pbar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} [{elapsed_precise}] {msg}")
                .unwrap(),
        );
        pbar.set_draw_target(ProgressDrawTarget::stderr_with_hz(20));

        Self {
            target_mapper,
            target_offset,
            geometry,
            local_stats,
            global_stats,
            barcode_buf: Vec::new(),
            umi_buf: Vec::new(),
            local_output_buf: Vec::new(),
            file,
            exact_matching,
            tid: 0,
            pbar: Arc::new(Mutex::new(pbar)),
            init_time,
            map_time: Instant::now(),
            adjustment,
            umi_quality_removal,
        }
    }

    fn encode_r1(&mut self, seq: &[u8]) -> Result<bool> {
        // clear encoding buffers
        self.barcode_buf.clear();
        self.umi_buf.clear();

        // Split R1 into barcode and UMI
        if seq.len() != self.geometry.barcode + self.geometry.umi {
            bail!("R1 sequence length does not match provided geometry");
        }
        let seq_bc = &seq[..self.geometry.barcode];
        let seq_umi = &seq[self.geometry.barcode..];

        // encode barcode and UMI
        if encode_with_invalid(seq_bc, &mut self.barcode_buf).is_err()
            || encode_with_invalid(seq_umi, &mut self.umi_buf).is_err()
        {
            return Ok(false);
        }

        if self.barcode_buf.len() != 1 || self.umi_buf.len() != 1 {
            bail!(
                "Barcode split assertion length failed - both barcode and UMI must be under 32bp"
            );
        }
        Ok(true)
    }

    fn statistics(&self) -> Statistics {
        let runtime = RuntimeStatistics::new(
            self.init_time.elapsed().as_secs_f64(),
            (self.map_time - self.init_time).as_secs_f64(),
            self.map_time.elapsed().as_secs_f64(),
            self.global_stats.lock().total_reads as f64 / self.map_time.elapsed().as_secs_f64(),
        );

        Statistics::new(
            LibraryCombination::Single(self.target_mapper.library_statistics()),
            self.global_stats.lock().to_owned(),
            runtime,
        )
    }

    fn map_sequence(&self, seq: &[u8]) -> Result<usize, MappingError> {
        if self.exact_matching {
            self.target_mapper
                .map(seq, self.target_offset, self.adjustment)
        } else {
            self.target_mapper
                .map_corrected(seq, self.target_offset, self.adjustment)
        }
    }

    fn write_buffer(&mut self) -> Result<()> {
        {
            let mut writer = self.file.lock();
            writer.write_all(&self.local_output_buf)?;
            writer.flush()?;
        }
        self.local_output_buf.clear();
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

    fn _process_record(&mut self, r1: &[u8], r2: &[u8], umi_qual: Option<&[u8]>) -> Result<()> {
        // Split R1 into barcode and UMI and 2-bit encode them
        if !self.encode_r1(r1)? {
            return Ok(()); // Skip the record if error on encoding barcode or UMI
        }

        // Shorthand the barcode and UMI
        let barcode = self.barcode_buf[0];
        let umi = self.umi_buf[0];

        if self.umi_quality_removal
            && let Some(umi_qual) = umi_qual
        {
            if umi_qual
                .iter()
                .any(|q| (*q - ILLUMINA_QUALITY_OFFSET) < UMI_MIN_QUALITY)
            {
                self.local_stats.increment_umi_qual_failure();
                return Ok(());
            }
        }

        // Map the sequence
        match self.map_sequence(r2) {
            Ok(index) => {
                // Write the record
                let record = ibu::Record::new(barcode, umi, index as u64);
                self.local_output_buf.write_all(record.as_bytes())?;
                self.local_stats.increment_mapped();
            }
            Err(why) => {
                self.local_stats.increment_unmapped(why);
            }
        }

        Ok(())
    }
}
impl<M: Mapper, Rf: paraseq::Record> PairedParallelProcessor<Rf> for MappingImplementor<M> {
    fn process_record_pair(&mut self, r1: Rf, r2: Rf) -> paraseq::parallel::Result<()> {
        self._process_record(
            &r1.seq(),
            &r2.seq(),
            r1.qual().map(|q| q.split_at(self.geometry.umi).1),
        )?;
        Ok(())
    }

    fn on_batch_complete(&mut self) -> paraseq::parallel::Result<()> {
        self.write_buffer()?;
        self.update_stats();
        self.update_pbar();
        Ok(())
    }

    fn set_thread_id(&mut self, thread_id: usize) {
        self.tid = thread_id;
    }
}

impl<M: Mapper> binseq::ParallelProcessor for MappingImplementor<M> {
    fn process_record<B: BinseqRecord>(&mut self, record: B) -> binseq::Result<()> {
        self._process_record(
            record.sseq(),
            record.xseq(),
            record
                .has_quality()
                .then(|| record.squal().split_at(self.geometry.umi).1),
        )?;
        Ok(())
    }

    fn on_batch_complete(&mut self) -> binseq::Result<()> {
        self.write_buffer()?;
        self.update_stats();
        self.update_pbar();
        Ok(())
    }

    fn set_tid(&mut self, tid: usize) {
        self.tid = tid;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn ibu_map_pairs_paraseq<M, R>(
    paired_readers: Vec<(fastx::Reader<R>, fastx::Reader<R>)>,
    filename: &str,
    target_mapper: Arc<M>,
    target_offset: Option<MapperOffset>,
    geometry: GeometryR1,
    num_threads: usize,
    exact_matching: bool,
    adjustment: bool,
    umi_quality_removal: bool,
    start_time: Instant,
) -> Result<Statistics>
where
    M: Mapper + 'static,
    R: Read + Send + 'static,
{
    // Initialize the header and write it to the output file
    let header = ibu::Header::try_from(geometry)?;
    let mut writer = open_file_handle(filename)?;
    writer.write_all(header.as_bytes())?;
    writer.flush()?;

    // Wrap the writer in an Arc<Mutex<W>>
    let writer = Arc::new(Mutex::new(writer));

    // Initialize the mapping implementor
    let implementor = MappingImplementor::new(
        target_mapper,
        target_offset,
        geometry,
        writer,
        exact_matching,
        adjustment,
        umi_quality_removal,
        start_time,
    );

    // Process the records in parallel
    let num_pairs = paired_readers.len();
    let threads_per_pair = num_threads / num_pairs;
    let mut handles = Vec::with_capacity(num_pairs);
    info!("Beginning mapping with {num_threads} threads over {num_pairs} file pairs",);
    for (rdr_r1, rdr_r2) in paired_readers {
        let mut implementor = implementor.clone();
        let handle = std::thread::spawn(move || -> Result<()> {
            rdr_r1.process_parallel_paired(rdr_r2, &mut implementor, threads_per_pair)?;
            Ok(())
        });
        handles.push(handle);
    }

    // Wait for all threads to finish
    for handle in handles {
        handle.join().unwrap()?;
    }

    // Finalize the progress bar
    implementor.finish_pbar();

    // Return the statistics
    Ok(implementor.statistics())
}

#[allow(clippy::too_many_arguments)]
pub fn ibu_map_pairs_binseq<M>(
    readers: Vec<BinseqReader>,
    filename: &str,
    target_mapper: Arc<M>,
    target_offset: Option<MapperOffset>,
    geometry: GeometryR1,
    num_threads: usize,
    exact_matching: bool,
    adjustment: bool,
    umi_quality_removal: bool,
    start_time: Instant,
) -> Result<Statistics>
where
    M: Mapper + 'static,
{
    // Initialize the header and write it to the output file
    let header = ibu::Header::try_from(geometry)?;
    let mut writer = open_file_handle(filename)?;
    writer.write_all(header.as_bytes())?;
    writer.flush()?;

    // Wrap the writer in an Arc<Mutex<W>>
    let writer = Arc::new(Mutex::new(writer));

    // Initialize the mapping implementor
    let implementor = MappingImplementor::new(
        target_mapper,
        target_offset,
        geometry,
        writer,
        exact_matching,
        adjustment,
        umi_quality_removal,
        start_time,
    );

    // Process the records in parallel
    info!(
        "Beginning mapping with {num_threads} threads over {} files",
        readers.len()
    );
    for reader in readers {
        reader.process_parallel(implementor.clone(), num_threads)?;
    }

    // Complete the progress bar
    implementor.finish_pbar();

    // Return the statistics
    Ok(implementor.statistics())
}
