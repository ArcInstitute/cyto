use std::io::Read;

use anyhow::{Result, bail};
use binseq::{BinseqRecord, bq::MmapReader};
use cyto_core::{
    Mapper,
    mappers::{GenericMapper, MapperOffset},
};
use paraseq::{Record, fastx::Reader};

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

pub fn find_offset_binseq(
    rdr: &MmapReader,
    mapper: &GenericMapper,
    max_records: usize,
) -> Result<MapperOffset> {
    let target_size = mapper.get_sequence_size();
    let seq_size = rdr.header().xlen as usize;

    let offset_scores = (0..seq_size - target_size)
        .map(|offset| -> Result<usize> {
            let mut n_match = 0;
            let mut dbuf = Vec::new();

            for idx in 0..max_records.min(rdr.num_records()) {
                dbuf.clear();
                let record = rdr.get(idx)?;
                record.decode_x(&mut dbuf)?;
                if mapper
                    .map(&dbuf, Some(MapperOffset::RightOf(offset)), None)
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

    Ok(MapperOffset::RightOf(max_idx))
}
