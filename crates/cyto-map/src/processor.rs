use std::{io::Write, sync::Arc, time::Instant};

use binseq::IntoBinseqError;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use paraseq::prelude::PairedParallelProcessor;
use parking_lot::Mutex;

use crate::{
    Bijection, BoxedWriter, Mapper, ProbeMapper, ReadMate, UmiMapper, WhitelistMapper,
    stats::MappingStatistics,
};

pub struct MapProcessor<M: Mapper> {
    /* Shared Resources */
    umi_mapper: UmiMapper,
    probe_mapper: Option<Arc<ProbeMapper>>,
    whitelist_mapper: Arc<WhitelistMapper>,
    feature_mapper: Arc<M>,
    bijection: Option<Arc<Bijection<String>>>,
    map_time: Instant,

    /* Local Variables */
    tid: usize,
    t_stats: MappingStatistics,
    t_output: Vec<Vec<u8>>,

    /* Global Variables */
    stats: Arc<Mutex<MappingStatistics>>,
    writers: Arc<Vec<Mutex<BoxedWriter>>>,
    pbar: Arc<Mutex<Option<ProgressBar>>>,
}
impl<M: Mapper> Clone for MapProcessor<M> {
    fn clone(&self) -> Self {
        Self {
            umi_mapper: self.umi_mapper,
            probe_mapper: self.probe_mapper.as_ref().map(Arc::clone),
            whitelist_mapper: Arc::clone(&self.whitelist_mapper),
            feature_mapper: Arc::clone(&self.feature_mapper),
            bijection: self.bijection.as_ref().map(Arc::clone),
            t_stats: self.t_stats,
            tid: self.tid,
            t_output: self.t_output.clone(),
            writers: Arc::clone(&self.writers),
            stats: Arc::clone(&self.stats),
            map_time: self.map_time,
            pbar: self.pbar.clone(),
        }
    }
}

impl<M: Mapper> MapProcessor<M> {
    /// Create a probed processor that demultiplexes output across multiple writers.
    pub fn probed(
        umi_mapper: UmiMapper,
        probe_mapper: ProbeMapper,
        whitelist_mapper: WhitelistMapper,
        feature_mapper: M,
        writers: Vec<BoxedWriter>,
        bijection: Bijection<String>,
    ) -> Self {
        let t_output = vec![Vec::default(); writers.len()];
        let shared_writers = writers.into_iter().map(Mutex::new).collect();
        Self {
            umi_mapper,
            probe_mapper: Some(Arc::new(probe_mapper)),
            whitelist_mapper: Arc::new(whitelist_mapper),
            feature_mapper: Arc::new(feature_mapper),
            bijection: Some(Arc::new(bijection)),
            t_stats: MappingStatistics::default(),
            t_output,
            writers: Arc::new(shared_writers),
            stats: Arc::new(Mutex::new(MappingStatistics::default())),
            tid: 0,
            map_time: Instant::now(),
            pbar: initialize_pbar(),
        }
    }

    /// Create an unprobed processor that writes all output to a single writer.
    pub fn unprobed(
        umi_mapper: UmiMapper,
        whitelist_mapper: WhitelistMapper,
        feature_mapper: M,
        writer: BoxedWriter,
    ) -> Self {
        Self {
            umi_mapper,
            probe_mapper: None,
            whitelist_mapper: Arc::new(whitelist_mapper),
            feature_mapper: Arc::new(feature_mapper),
            bijection: None,
            t_stats: MappingStatistics::default(),
            t_output: vec![Vec::default()],
            writers: Arc::new(vec![Mutex::new(writer)]),
            stats: Arc::new(Mutex::new(MappingStatistics::default())),
            tid: 0,
            map_time: Instant::now(),
            pbar: initialize_pbar(),
        }
    }

    pub fn pprint(&self) {
        let stats = *self.stats.lock();
        println!(
            "Mapped: {} / {} ({:.2}%)",
            stats.mapped_reads,
            stats.total_reads,
            stats.frac_mapped() * 100.0,
        );

        println!("{stats:#?}");
    }
    pub fn total(&self) -> usize {
        self.stats.lock().total_reads
    }
    pub fn stats(&self) -> MappingStatistics {
        *self.stats.lock()
    }

    fn update_pbar(&self) {
        if self.tid == 0 {
            let (total, map_pc) = {
                let stats = self.stats.lock();
                (
                    stats.total_reads as f64 / 1_000_000.0,
                    stats.mapped_reads as f64 / stats.total_reads as f64 * 100.0,
                )
            };
            let elapsed = self.map_time.elapsed().as_secs_f64();
            let throughput = total / elapsed;
            {
                if let Some(pb) = self.pbar.lock().as_mut() {
                    pb.set_message(format!(
                        "Processed: {total:.3}M reads ( Mapped: {map_pc:.2}%, Throughput: {throughput:.3}M/s )",
                    ));
                }
            }
        }
    }

    pub fn finish_pbar(&mut self) {
        let (total, map_pc) = {
            let stats = self.stats.lock();
            (
                stats.total_reads as f64 / 1_000_000.0,
                stats.mapped_reads as f64 / stats.total_reads as f64 * 100.0,
            )
        };
        let elapsed = self.map_time.elapsed().as_secs_f64();
        let throughput = total / elapsed;

        {
            if let Some(pb) = self.pbar.lock().as_mut() {
                pb.finish_with_message(format!(
                        "Mapping complete: {total:.3}M reads ( Mapped: {map_pc:.2}%, Throughput: {throughput:.3}M/s )",
                    ));
            }
        }
    }

    fn increment_missing(
        &mut self,
        probe_missing: bool,
        feat_idx: Option<usize>,
        wl_idx: Option<usize>,
        umi: Option<u64>,
        pass_qual: bool,
    ) {
        if probe_missing {
            self.t_stats.unmapped.missing_probe += 1;
        }
        feat_idx
            .is_none()
            .then(|| self.t_stats.unmapped.missing_feature += 1);
        wl_idx
            .is_none()
            .then(|| self.t_stats.unmapped.missing_whitelist += 1);
        umi.is_none()
            .then(|| self.t_stats.unmapped.umi_truncated += 1);
        (!pass_qual).then(|| self.t_stats.unmapped.failed_umi_qual += 1);
    }

    fn _process_record(
        &mut self,
        s_seq: &[u8],
        x_seq: &[u8],
        s_qual: &[u8],
        x_qual: &[u8],
    ) -> anyhow::Result<()> {
        self.t_stats.total_reads += 1;

        // query feature and whitelist mappers
        let feat_match =
            self.feature_mapper
                .query(select_mate(s_seq, x_seq, self.feature_mapper.mate()));
        let wl_match =
            self.whitelist_mapper
                .query(select_mate(s_seq, x_seq, self.whitelist_mapper.mate()));

        let feat_idx = feat_match.map(|m| m.feature_idx);
        let wl_idx = wl_match.map(|m| m.feature_idx);

        let umi = match self.umi_mapper.extract_2bit_umi(select_mate(
            s_seq,
            x_seq,
            self.umi_mapper.mate(),
        )) {
            Some(res) => Some(res?),
            None => None,
        };

        let pass_qual = self.umi_mapper.passes_quality_threshold(select_mate(
            s_qual,
            x_qual,
            self.umi_mapper.mate(),
        ));

        // resolve output index: probe demux or single output
        let output_idx =
            if let (Some(probe_mapper), Some(bijection)) = (&self.probe_mapper, &self.bijection) {
                let probe_seq = select_mate(s_seq, x_seq, probe_mapper.mate());
                let probe_match = if probe_mapper.is_dynamic() {
                    // Probe follows a variable-length component: compute its actual
                    // offset from the feature mapper's match end position.
                    //
                    // Can only be done if the feature mapper has a match.
                    feat_match.and_then(|m| {
                        probe_mapper.query_at(probe_seq, m.end_pos + probe_mapper.dynamic_offset())
                    })
                } else {
                    probe_mapper.query(probe_seq)
                };
                let Some(p_match) = probe_match else {
                    // Short-circuit on probe miss
                    self.increment_missing(true, feat_idx, wl_idx, umi, pass_qual);
                    return Ok(());
                };
                Some(
                    probe_mapper
                        .get_parent(p_match.feature_idx)
                        .map(|seq| bijection.get_index(seq))
                        .expect("Failed to recover probe index")
                        .expect("Failed to biject probe parent sequence"),
                )
            } else {
                // No probe mapper available
                None
            };

        // handle match conditions
        if let (Some(f_idx), Some(wl_idx), Some(umi), true) = (feat_idx, wl_idx, umi, pass_qual) {
            self.t_stats.mapped_reads += 1;

            let bc = self
                .whitelist_mapper
                .get_parent_2bit(wl_idx)
                .expect("Failed to get whitelist parent index when expected in bounds")
                .map_err(IntoBinseqError::into_binseq_error)?;

            let ibu = ibu::Record::new(bc, umi, f_idx as u64);
            let idx = output_idx.unwrap_or(0);

            self.t_output
                .get_mut(idx)
                .expect("Failed to get mutable reference to output head")
                .write_all(ibu.as_bytes())?;
        } else {
            self.increment_missing(false, feat_idx, wl_idx, umi, pass_qual);
        }
        Ok(())
    }

    fn _on_batch_complete(&mut self) -> anyhow::Result<()> {
        // write local (in-memory) outputs to global outputs
        {
            // Write all local output buffers to the corresponding files
            for idx in 0..self.t_output.len() {
                // Scope the lock to ensure it is released early
                {
                    let writer = &mut self.writers[idx].lock();
                    writer.write_all(&self.t_output[idx])?;
                    writer.flush()?;
                }
                self.t_output[idx].clear();
            }
        }

        // update statistics
        {
            *self.stats.lock() += self.t_stats;
            self.t_stats.clear();
        }

        // update pbar
        self.update_pbar();

        Ok(())
    }
}

fn select_mate<'a>(m1: &'a [u8], m2: &'a [u8], mate: ReadMate) -> &'a [u8] {
    match mate {
        ReadMate::R1 => m1,
        ReadMate::R2 => m2,
    }
}

impl<M: Mapper + Send + Sync> binseq::ParallelProcessor for MapProcessor<M> {
    fn process_record<R: binseq::BinseqRecord>(&mut self, record: R) -> binseq::Result<()> {
        self._process_record(record.sseq(), record.xseq(), record.squal(), record.xqual())?;
        Ok(())
    }
    fn on_batch_complete(&mut self) -> binseq::Result<()> {
        self._on_batch_complete()?;
        Ok(())
    }
    fn set_tid(&mut self, tid: usize) {
        self.tid = tid;
    }
    fn get_tid(&self) -> Option<usize> {
        Some(self.tid)
    }
}

impl<M: Mapper + Send + Sync, Rf: paraseq::Record> PairedParallelProcessor<Rf> for MapProcessor<M> {
    fn process_record_pair(&mut self, record1: Rf, record2: Rf) -> paraseq::Result<()> {
        self._process_record(
            record1.seq().as_ref(),
            record2.seq().as_ref(),
            record1.qual().unwrap_or_default(), // TODO: handle potentially missing quality scores
            record2.qual().unwrap_or_default(),
        )?;
        Ok(())
    }
    fn on_batch_complete(&mut self) -> paraseq::Result<()> {
        self._on_batch_complete()?;
        Ok(())
    }
    fn set_thread_id(&mut self, thread_id: usize) {
        self.tid = thread_id;
    }
    fn get_thread_id(&self) -> usize {
        self.tid
    }
}

fn initialize_pbar() -> Arc<Mutex<Option<ProgressBar>>> {
    let pbar = ProgressBar::new_spinner();
    pbar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} [{elapsed_precise}] {msg}")
            .unwrap(),
    );
    pbar.set_draw_target(ProgressDrawTarget::stderr_with_hz(20));
    Arc::new(Mutex::new(Some(pbar)))
}
