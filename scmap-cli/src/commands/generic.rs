use anyhow::Result;
use scmap::{
    mappers::MapperOffset,
    statistics::{LibraryCombination, Statistics},
    GeometryR1, Mapper, MappingStatistics, PairedReader,
};

use crate::progress::ProgressBar;

pub fn map_pairs<M>(
    mut reader: PairedReader,
    target_mapper: &M,
    target_offset: Option<MapperOffset>,
    geometry: GeometryR1,
) -> Result<Statistics>
where
    M: Mapper,
{
    let lib_stats = LibraryCombination::Single(target_mapper.library_statistics());
    let mut map_stats = MappingStatistics::default();
    let mut pbar = ProgressBar::default();
    while let Some(pair) = reader.next() {
        let pair = pair?;
        let Ok(bus) = pair.as_bus(geometry.barcode, geometry.umi) else {
            continue;
        };
        match target_mapper.map(bus.seq, target_offset) {
            Ok(_index) => {
                map_stats.increment_mapped();
            }
            Err(why) => map_stats.increment_unmapped(why),
        }
        pbar.tick();
    }
    pbar.finish();
    Ok(Statistics::new(lib_stats, map_stats))
}

pub fn map_probed_pairs<Mt, Mp>(
    mut reader: PairedReader,
    target_mapper: &Mt,
    probe_mapper: &Mp,
    target_offset: Option<MapperOffset>,
    probe_offset: Option<MapperOffset>,
    geometry: GeometryR1,
) -> Result<Statistics>
where
    Mt: Mapper,
    Mp: Mapper,
{
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
            (Ok(_t_idx), Ok(_p_idx)) => {
                map_stats.increment_mapped();
            }
            (Err(why), Ok(_)) | (Ok(_), Err(why)) => map_stats.increment_unmapped(why),
            (Err(why1), Err(why2)) => map_stats.increment_unmapped_multi_reason(why1, why2),
        }
        pbar.tick();
    }
    pbar.finish();
    Ok(Statistics::new(lib_stats, map_stats))
}
