use std::{io::Write, ops::AddAssign, sync::Arc};

use binseq::{IntoBinseqError, ParallelProcessor};
use parking_lot::Mutex;

use crate::v2::{
    Bijection, BoxedWriter, Mapper, ProbeMapper, ReadMate, UmiMapper, WhitelistMapper,
};

pub struct MapProcessor<M: Mapper> {
    umi_mapper: UmiMapper,
    probe_mapper: Arc<ProbeMapper>,
    whitelist_mapper: Arc<WhitelistMapper>,
    feature_mapper: Arc<M>,
    t_mapped: usize,
    t_total: usize,
    t_output: Vec<Vec<u8>>,
    bijection: Arc<Bijection<String>>,
    writers: Arc<Vec<Mutex<BoxedWriter>>>,
    mapped: Arc<Mutex<usize>>,
    total: Arc<Mutex<usize>>,
}
impl<M: Mapper> Clone for MapProcessor<M> {
    fn clone(&self) -> Self {
        Self {
            umi_mapper: self.umi_mapper,
            probe_mapper: Arc::clone(&self.probe_mapper),
            whitelist_mapper: Arc::clone(&self.whitelist_mapper),
            feature_mapper: Arc::clone(&self.feature_mapper),
            t_mapped: 0,
            t_total: 0,
            t_output: self.t_output.clone(),
            bijection: Arc::clone(&self.bijection),
            writers: Arc::clone(&self.writers),
            mapped: Arc::clone(&self.mapped),
            total: Arc::clone(&self.total),
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
            t_mapped: 0,
            t_total: 0,
            t_output: t_output,
            bijection: Arc::new(bijection),
            writers: Arc::new(shared_writers),
            mapped: Arc::new(Mutex::new(0)),
            total: Arc::new(Mutex::new(0)),
        }
    }
    pub fn pprint(&self) {
        let mapped = *self.mapped.lock();
        let total = *self.total.lock();
        println!(
            "Mapped: {} / {} ({:.2}%)",
            mapped,
            total,
            mapped as f64 / total as f64 * 100.0
        );
    }
    pub fn total(&self) -> usize {
        *self.total.lock()
    }
}

fn select_seq<'a, R: binseq::BinseqRecord>(record: &'a R, mate: ReadMate) -> &'a [u8] {
    match mate {
        ReadMate::R1 => record.sseq(),
        ReadMate::R2 => record.xseq(),
    }
}

impl<M: Mapper + Send + Sync> ParallelProcessor for MapProcessor<M> {
    fn process_record<R: binseq::BinseqRecord>(&mut self, record: R) -> binseq::Result<()> {
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

        // handle match conditions
        match (probe_idx, feat_idx, wl_idx) {
            (Some(p_idx), Some(f_idx), Some(wl_idx)) => {
                // convert barcode
                let bc = self
                    .whitelist_mapper
                    .get_parent_2bit(wl_idx)
                    .expect("Failed to get whitelist parent index when expected in bounds")
                    .map_err(IntoBinseqError::into_binseq_error)?;

                // convert umi (maps 'N' -> 'A')
                let umi = match self
                    .umi_mapper
                    .extract_2bit_umi(select_seq(&record, self.umi_mapper.mate()))
                {
                    Some(res) => res?,
                    None => {
                        eprintln!("UMI out of range of record");
                        self.t_total += 1;
                        return Ok(());
                    }
                };

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

                self.t_mapped += 1;
            }
            _ => {}
        }

        // increment total
        self.t_total += 1;
        Ok(())
    }
    fn on_batch_complete(&mut self) -> binseq::Result<()> {
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

        self.mapped.lock().add_assign(self.t_mapped);
        self.total.lock().add_assign(self.t_total);

        {
            self.t_mapped = 0;
            self.t_total = 0;
        }
        Ok(())
    }
}
