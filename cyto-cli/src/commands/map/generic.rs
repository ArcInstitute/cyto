use anyhow::Result;
use cyto::{
    mappers::{MapperOffset, ProbeMapper},
    statistics::{LibraryCombination, Statistics},
    GeometryR1, Mapper, MappingStatistics, PairedReader,
};
use ibu::{Header, Record};
use std::io::Write;

use crate::progress::ProgressBar;

pub fn ibu_map_pairs<M, W>(
    mut reader: PairedReader,
    writer: &mut W,
    target_mapper: &M,
    target_offset: Option<MapperOffset>,
    geometry: GeometryR1,
) -> Result<Statistics>
where
    M: Mapper,
    W: Write,
{
    // Initialize the header and write it to the output file
    let header = Header::try_from(geometry)?;
    header.write_bytes(writer)?;

    // Initialize the statistics
    let lib_stats = LibraryCombination::Single(target_mapper.library_statistics());
    let mut map_stats = MappingStatistics::default();

    // Initialize the main loop
    let mut pbar = ProgressBar::default();
    while let Some(pair) = reader.next() {
        let pair = pair?;
        let Ok(bus) = pair.as_bus(geometry.barcode, geometry.umi) else {
            continue;
        };
        match target_mapper.map(bus.seq, target_offset) {
            Ok(index) => {
                let record = Record::new(bus.barcode, bus.umi, index as u64);
                record.write_bytes(writer)?;
                map_stats.increment_mapped();
            }
            Err(why) => map_stats.increment_unmapped(why),
        }
        pbar.tick();
    }
    pbar.finish();
    writer.flush()?;
    Ok(Statistics::new(lib_stats, map_stats))
}

pub fn ibu_map_probed_pairs<M, W>(
    mut reader: PairedReader,
    writers: &mut [W],
    target_mapper: &M,
    probe_mapper: &ProbeMapper,
    target_offset: Option<MapperOffset>,
    probe_offset: Option<MapperOffset>,
    geometry: GeometryR1,
) -> Result<Statistics>
where
    M: Mapper,
    W: Write,
{
    // Initialize the header and write it to the output file
    let header = Header::try_from(geometry)?;
    for writer in writers.iter_mut() {
        header.write_bytes(writer)?;
    }

    // Initialize the statistics
    let lib_stats = LibraryCombination::Dual(
        target_mapper.library_statistics(),
        probe_mapper.library_statistics(),
    );
    let mut map_stats = MappingStatistics::default();
    let mut pbar = ProgressBar::default();
    while let Some(pair) = reader.next() {
        let pair = pair?;
        let Ok(bus) = pair.as_bus(geometry.barcode, geometry.umi) else {
            continue;
        };
        let target_index = target_mapper.map(bus.seq, target_offset);
        let probe_index = probe_mapper.map(bus.seq, probe_offset);
        match (target_index, probe_index) {
            (Ok(t_idx), Ok(p_idx)) => {
                // Create the record
                let record = Record::new(bus.barcode, bus.umi, t_idx as u64);

                // Write the record to the correct file
                let probe_alias_index = probe_mapper.get_alias_index(p_idx).unwrap();
                record.write_bytes(&mut writers[probe_alias_index])?;

                // Update the mapping statistics
                map_stats.increment_mapped();
            }
            (Err(why), Ok(_)) | (Ok(_), Err(why)) => map_stats.increment_unmapped(why),
            (Err(why1), Err(why2)) => map_stats.increment_unmapped_multi_reason(why1, why2),
        }
        pbar.tick();
    }
    pbar.finish();

    // Flush all writers
    for writer in writers.iter_mut() {
        writer.flush()?;
    }

    Ok(Statistics::new(lib_stats, map_stats))
}
