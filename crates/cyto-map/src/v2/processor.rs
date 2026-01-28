use std::{io::Write, sync::Arc};

use binseq::{IntoBinseqError, ParallelProcessor};
use parking_lot::Mutex;

use crate::v2::{
    Bijection, BoxedWriter, Mapper, ProbeMapper, ReadMate, UmiMapper, WhitelistMapper,
    stats::MappingStatistics,
};

pub struct MapProcessor<M: Mapper> {
    /* Shared Resources */
    umi_mapper: UmiMapper,
    probe_mapper: Arc<ProbeMapper>,
    whitelist_mapper: Arc<WhitelistMapper>,
    feature_mapper: Arc<M>,
    bijection: Arc<Bijection<String>>,

    /* Local Variables */
    t_stats: MappingStatistics,
    t_output: Vec<Vec<u8>>,

    /* Global Variables */
    stats: Arc<Mutex<MappingStatistics>>,
    writers: Arc<Vec<Mutex<BoxedWriter>>>,
}
impl<M: Mapper> Clone for MapProcessor<M> {
    fn clone(&self) -> Self {
        Self {
            umi_mapper: self.umi_mapper,
            probe_mapper: Arc::clone(&self.probe_mapper),
            whitelist_mapper: Arc::clone(&self.whitelist_mapper),
            feature_mapper: Arc::clone(&self.feature_mapper),
            t_stats: self.t_stats,
            t_output: self.t_output.clone(),
            bijection: Arc::clone(&self.bijection),
            writers: Arc::clone(&self.writers),
            stats: Arc::clone(&self.stats),
        }
    }
}

impl<M: Mapper> MapProcessor<M> {
    pub fn new(
        umi_mapper: UmiMapper,
        probe_mapper: ProbeMapper,
        whitelist_mapper: WhitelistMapper,
        feature_mapper: M,
        writers: Vec<BoxedWriter>,
        bijection: Bijection<String>,
    ) -> Self {
        let t_output = vec![Vec::default(); writers.len()];
        let shared_writers = writers.into_iter().map(|w| Mutex::new(w)).collect();
        Self {
            umi_mapper,
            probe_mapper: Arc::new(probe_mapper),
            whitelist_mapper: Arc::new(whitelist_mapper),
            feature_mapper: Arc::new(feature_mapper),
            t_stats: MappingStatistics::default(),
            t_output: t_output,
            bijection: Arc::new(bijection),
            writers: Arc::new(shared_writers),
            stats: Arc::new(Mutex::new(MappingStatistics::default())),
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

        println!("{:#?}", stats);
    }
    pub fn total(&self) -> usize {
        self.stats.lock().total_reads
    }
}

fn select_seq<'a, R: binseq::BinseqRecord>(record: &'a R, mate: ReadMate) -> &'a [u8] {
    match mate {
        ReadMate::R1 => record.sseq(),
        ReadMate::R2 => record.xseq(),
    }
}

fn select_qual<'a, R: binseq::BinseqRecord>(record: &'a R, mate: ReadMate) -> &'a [u8] {
    match mate {
        ReadMate::R1 => record.squal(),
        ReadMate::R2 => record.xqual(),
    }
}

impl<M: Mapper + Send + Sync> ParallelProcessor for MapProcessor<M> {
    fn process_record<R: binseq::BinseqRecord>(&mut self, record: R) -> binseq::Result<()> {
        // increment total reads
        self.t_stats.total_reads += 1;

        // query each mapper
        let probe_idx = self
            .probe_mapper
            .query(select_seq(&record, self.probe_mapper.mate()));
        let feat_idx = self
            .feature_mapper
            .query(select_seq(&record, self.feature_mapper.mate()));
        let wl_idx = self
            .whitelist_mapper
            .query(select_seq(&record, self.whitelist_mapper.mate()));

        let umi = match self
            .umi_mapper
            .extract_2bit_umi(select_seq(&record, self.umi_mapper.mate()))
        {
            Some(res) => Some(res?),
            None => None,
        };

        let pass_qual = self
            .umi_mapper
            .passes_quality_threshold(select_qual(&record, self.umi_mapper.mate()));

        // handle match conditions
        match (probe_idx, feat_idx, wl_idx, umi, pass_qual) {
            (Some(p_idx), Some(f_idx), Some(wl_idx), Some(umi), true) => {
                // increment mapped reads
                self.t_stats.mapped_reads += 1;

                // convert barcode
                let bc = self
                    .whitelist_mapper
                    .get_parent_2bit(wl_idx)
                    .expect("Failed to get whitelist parent index when expected in bounds")
                    .map_err(IntoBinseqError::into_binseq_error)?;

                // build IBU record
                let ibu = ibu::Record::new(bc, umi, f_idx as u64);

                // identify correct output head
                let output_idx = self
                    .probe_mapper
                    .get_parent(p_idx)
                    .map(|seq| self.bijection.get_index(seq))
                    .expect("Failed to recover probe index")
                    .expect("Failed to biject probe parent sequence");

                self.t_output
                    .get_mut(output_idx)
                    .expect("Failed to get mutable reference to output head")
                    .write_all(&ibu.as_bytes())?;
            }
            _ => {
                probe_idx
                    .is_none()
                    .then(|| self.t_stats.unmapped.missing_probe += 1);
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
        }
        Ok(())
    }
    fn on_batch_complete(&mut self) -> binseq::Result<()> {
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

        Ok(())
    }
}
