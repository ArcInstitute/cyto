use std::io::Read;
use std::sync::Arc;

use anyhow::{Result, bail};
use binseq::prelude::*;
use cyto_core::{
    Mapper,
    mappers::{GenericMapper, MapperOffset},
};
use paraseq::{Record, fastx::Reader};
use parking_lot::Mutex;

fn argmax(vec: &[usize]) -> usize {
    let mut max_idx = 0;
    let mut max_score = 0;
    for (idx, score) in vec.iter().enumerate() {
        if *score > max_score {
            max_idx = idx;
            max_score = *score;
        }
    }
    max_idx
}

pub fn find_offset_paraseq<R: Read>(
    rdr: &mut Reader<R>,
    mapper: &GenericMapper,
) -> Result<MapperOffset> {
    let mut rset = rdr.new_record_set();
    if !rset.fill(rdr)? {
        bail!("Empty record set");
    }
    let target_size = mapper.get_sequence_size();
    let seq_size = rset.iter().next().unwrap()?.seq().len();

    let offset_scores = (0..seq_size - target_size)
        .map(|offset| -> Result<usize> {
            let mut n_match = 0;
            for record in rset.iter() {
                let record = record?;
                if mapper
                    .map(&record.seq(), Some(MapperOffset::RightOf(offset)), None)
                    .is_ok()
                {
                    n_match += 1;
                }
            }
            Ok(n_match)
        })
        .collect::<Result<Vec<_>>>()?;

    let max_idx = argmax(&offset_scores);
    eprintln!("Found best matching rate at position: {max_idx}");

    // reload the Reader
    rdr.reload(&mut rset)?;

    Ok(MapperOffset::RightOf(max_idx))
}

#[derive(Clone, Default)]
pub struct MaxSeqSize {
    max: Arc<Mutex<usize>>,
}
impl MaxSeqSize {
    pub fn get(&self) -> usize {
        *self.max.lock()
    }
    pub fn run(rdr: BinseqReader, max_records: usize) -> Result<usize> {
        let max_seq_size = MaxSeqSize::default();
        rdr.process_parallel_range(max_seq_size.clone(), 1, 0..max_records)?;
        Ok(max_seq_size.get())
    }
}
impl ParallelProcessor for MaxSeqSize {
    fn process_record<R: BinseqRecord>(&mut self, record: R) -> binseq::Result<()> {
        let mut tmp = self.max.lock();
        *tmp = tmp.max(record.xlen() as usize);
        Ok(())
    }
}

#[derive(Clone)]
pub struct BestOffset {
    mapper: Arc<GenericMapper>,
    max_seq_size: usize,
    target_size: usize,
    offset_scores: Arc<Mutex<Vec<usize>>>,
    dbuf: Vec<u8>,
}
impl BestOffset {
    pub fn new(mapper: Arc<GenericMapper>, max_seq_size: usize, target_size: usize) -> Self {
        BestOffset {
            mapper,
            max_seq_size,
            target_size,
            offset_scores: Arc::new(Mutex::new(vec![0; max_seq_size - target_size])),
            dbuf: Vec::new(),
        }
    }
    pub fn get_best_offset(&self) -> usize {
        let max_idx = argmax(&self.offset_scores.lock());
        max_idx
    }
    pub fn run(
        rdr: BinseqReader,
        max_records: usize,
        mapper: Arc<GenericMapper>,
        max_seq_size: usize,
    ) -> Result<usize> {
        let target_size = mapper.get_sequence_size();
        if target_size > max_seq_size {
            bail!(
                "Target size ({}) is greater than the maximum sequence size ({}) of the first ({}) records",
                target_size,
                max_seq_size,
                max_records
            )
        }
        let best_offset = BestOffset::new(mapper, max_seq_size, target_size);
        rdr.process_parallel_range(best_offset.clone(), 1, 0..max_records)?;
        let best_offset = best_offset.get_best_offset();
        Ok(best_offset)
    }
}
impl ParallelProcessor for BestOffset {
    fn process_record<R: BinseqRecord>(&mut self, record: R) -> binseq::Result<()> {
        self.dbuf.clear();
        record.decode_x(&mut self.dbuf)?;

        for offset in 0..self.max_seq_size - self.target_size {
            self.mapper
                .map(&self.dbuf, Some(MapperOffset::RightOf(offset)), None)
                .is_ok()
                .then(|| self.offset_scores.lock()[offset] += 1);
        }

        Ok(())
    }
}

pub fn find_offset_binseq(
    path: &str,
    mapper: Arc<GenericMapper>,
    max_records: usize,
) -> Result<MapperOffset> {
    let max_seq_size = MaxSeqSize::run(BinseqReader::new(path)?, max_records)?;
    eprintln!("Maximum sequence size of first {max_records} records: {max_seq_size}");
    let max_idx = BestOffset::run(BinseqReader::new(path)?, max_records, mapper, max_seq_size)?;
    eprintln!("Found best matching rate at position: {max_idx}");
    Ok(MapperOffset::RightOf(max_idx))
}
