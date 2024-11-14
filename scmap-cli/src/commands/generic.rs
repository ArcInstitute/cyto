use anyhow::Result;
use scmap::{
    mappers::MapperOffset,
    statistics::{LibraryCombination, Statistics},
    BarcodeIndexCounter, BusCounter, Counter, GeometryR1, Mapper, MappingStatistics, PairedReader,
    ProbeBarcodeIndexCounter, ProbeBusCounter,
};

use crate::progress::ProgressBar;

pub fn map_pairs<M>(
    mut reader: PairedReader,
    target_mapper: &M,
    target_offset: Option<MapperOffset>,
    geometry: GeometryR1,
) -> Result<(BarcodeIndexCounter, Statistics)>
where
    M: Mapper,
{
    let mut counter = BusCounter::default();
    let lib_stats = LibraryCombination::Single(target_mapper.library_statistics());
    let mut map_stats = MappingStatistics::default();
    let mut pbar = ProgressBar::default();
    while let Some(pair) = reader.next() {
        let pair = pair?;
        let bus = pair.as_bus(geometry.barcode, geometry.umi);
        match target_mapper.map(&bus.seq, target_offset) {
            Ok(index) => {
                counter.increment(&bus, index);
                map_stats.increment_mapped();
            }
            Err(why) => map_stats.increment_unmapped(why),
        }
        pbar.tick();
    }
    pbar.finish();
    let statistics = Statistics::new(lib_stats, map_stats);
    Ok((counter.dedup_umi(), statistics))
}

pub fn map_probed_pairs<Mt, Mp>(
    mut reader: PairedReader,
    target_mapper: &Mt,
    probe_mapper: &Mp,
    target_offset: Option<MapperOffset>,
    probe_offset: Option<MapperOffset>,
    geometry: GeometryR1,
) -> Result<(ProbeBarcodeIndexCounter, Statistics)>
where
    Mt: Mapper,
    Mp: Mapper,
{
    let mut counter = ProbeBusCounter::default();
    let lib_stats = LibraryCombination::Dual(
        target_mapper.library_statistics(),
        probe_mapper.library_statistics(),
    );
    let mut map_stats = MappingStatistics::default();
    let mut pbar = ProgressBar::default();
    while let Some(pair) = reader.next() {
        let pair = pair?;
        let bus = pair.as_bus(geometry.barcode, geometry.umi);
        let target_index = target_mapper.map(&bus.seq, target_offset);
        let probe_index = probe_mapper.map(&bus.seq, probe_offset);
        match (target_index, probe_index) {
            (Ok(t_idx), Ok(p_idx)) => {
                counter.increment_probe(p_idx, &bus, t_idx);
                map_stats.increment_mapped();
            }
            (Err(why), Ok(_)) => map_stats.increment_unmapped(why),
            (Ok(_), Err(why)) => map_stats.increment_unmapped(why),
            (Err(why1), Err(why2)) => map_stats.increment_unmapped_multi_reason(why1, why2),
        }
        pbar.tick();
    }
    pbar.finish();
    let statistics = Statistics::new(lib_stats, map_stats);
    Ok((counter.dedup_umi(), statistics))
}
