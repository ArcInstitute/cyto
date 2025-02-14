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
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use paraseq::{
    fastq::Reader,
    fastx::Record as FastxRecord,
    parallel::{PairedParallelProcessor, PairedParallelReader},
};
use parking_lot::Mutex;

use crate::io::open_file_handle;

#[cfg(feature = "binseq")]
use binseq::{PairedMmapReader, ParallelPairedProcessor};

#[derive(Clone)]
pub struct MappingImplementor<M: Mapper> {
    target_mapper: M,
    target_offset: Option<MapperOffset>,
    geometry: GeometryR1,

    local_stats: MappingStatistics,
    global_stats: Arc<Mutex<MappingStatistics>>,

    // Buffers for barcode and UMI sequences used during splitting
    barcode_buf: Vec<u64>,
    umi_buf: Vec<u64>,

    // Temporary decoding buffer for R2 sequences (binseq)
    #[cfg(feature = "binseq")]
    dbuf: Vec<u8>,

    // Temporary buffer for output
    local_output_buf: Vec<u8>,

    // Output file name
    // filename: String,
    file: Arc<Mutex<Box<dyn Write + Send>>>,

    // Exact matching flag
    exact_matching: bool,

    // thread id
    tid: usize,

    // progress bar
    pbar: Arc<Mutex<ProgressBar>>,
}
impl<M: Mapper> MappingImplementor<M> {
    pub fn new(
        target_mapper: M,
        target_offset: Option<MapperOffset>,
        geometry: GeometryR1,
        file: Arc<Mutex<Box<dyn Write + Send>>>,
        exact_matching: bool,
    ) -> Self {
        let local_stats = MappingStatistics::default();
        let global_stats = Arc::new(Mutex::new(MappingStatistics::default()));

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
            #[cfg(feature = "binseq")]
            dbuf: Vec::new(),
            local_output_buf: Vec::new(),
            file,
            exact_matching,
            tid: 0,
            pbar: Arc::new(Mutex::new(pbar)),
        }
    }

    fn encode_r1<R: FastxRecord>(&mut self, record: &R) -> Result<bool> {
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

    #[cfg(feature = "binseq")]
    fn split_r1(&mut self, pair: &binseq::RefRecordPair) -> Result<()> {
        // Split R1 into barcode and UMI
        let r1_config = pair.s_config();
        if r1_config.slen as usize != self.geometry.barcode + self.geometry.umi {
            bail!("R1 sequence length does not match provided geometry");
        }
        bitnuc::split_packed(
            pair.s_seq(),
            r1_config.slen as usize,
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

    #[cfg(feature = "binseq")]
    fn decode_r2(&mut self, pair: &binseq::RefRecordPair) -> Result<()> {
        self.dbuf.clear();
        bitnuc::decode(pair.x_seq, pair.x_config.slen as usize, &mut self.dbuf)?;
        Ok(())
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
            let pb = self.pbar.lock();
            let throughput = total / pb.elapsed().as_secs_f64();
            pb.set_message(format!(
                "Processed: {:.3}M reads ( Mapped: {:.2}%, Throughput: {:.3}M/s )",
                total, map_pc, throughput
            ));
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
        let pb = self.pbar.lock();
        let throughput = total / pb.elapsed().as_secs_f64();
        pb.finish_with_message(format!(
            "Mapping complete: {:.3}M reads ( Mapped: {:.2}%, Throughput: {:.3}M/s )",
            total, map_pc, throughput
        ));
    }
}
impl<M: Mapper> PairedParallelProcessor for MappingImplementor<M> {
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

        // Shorthand the barcode and UMI
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
        self.write_buffer()?;
        self.update_stats();
        self.update_pbar();
        Ok(())
    }

    fn set_thread_id(&mut self, thread_id: usize) {
        self.tid = thread_id;
    }
}

#[cfg(feature = "binseq")]
impl<M: Mapper> ParallelPairedProcessor for MappingImplementor<M> {
    fn process_record_pair(&mut self, pair: binseq::RefRecordPair) -> Result<()> {
        // Split R1 into barcode and UMI
        self.split_r1(&pair)?;

        // Decode R2
        self.decode_r2(&pair)?;

        // Shorthand the barcode and UMI
        let barcode = self.barcode_buf[0];
        let umi = self.umi_buf[0];

        // Map the sequence
        match self.map_sequence(&self.dbuf) {
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

    fn on_batch_complete(&mut self) -> Result<()> {
        self.write_buffer()?;
        self.update_stats();
        self.update_pbar();
        Ok(())
    }

    fn set_tid(&mut self, tid: usize) {
        self.tid = tid;
    }
}

pub fn ibu_map_pairs_paraseq<M, R>(
    rdr_r1: Reader<R>,
    rdr_r2: Reader<R>,
    filename: &str,
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
    let header = ibu::Header::try_from(geometry)?;
    let mut writer = open_file_handle(filename)?;
    header.write_bytes(&mut writer)?;
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
    );

    // Process the records in parallel
    rdr_r1.process_parallel_paired(rdr_r2, implementor.clone(), num_threads)?;

    // Finalize the progress bar
    implementor.finish_pbar();

    // Return the statistics
    Ok(implementor.statistics())
}

#[cfg(feature = "binseq")]
pub fn ibu_map_pairs_binseq<M>(
    reader: PairedMmapReader,
    filename: &str,
    target_mapper: M,
    target_offset: Option<MapperOffset>,
    geometry: GeometryR1,
    num_threads: usize,
    exact_matching: bool,
) -> Result<Statistics>
where
    M: Mapper + 'static,
{
    // Initialize the header and write it to the output file
    let header = ibu::Header::try_from(geometry)?;
    let mut writer = open_file_handle(filename)?;
    header.write_bytes(&mut writer)?;
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
    );

    // Process the records in parallel
    reader.process_parallel(implementor.clone(), num_threads)?;

    // Complete the progress bar
    implementor.finish_pbar();

    // Return the statistics
    Ok(implementor.statistics())
}
