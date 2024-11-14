use anyhow::Result;
use scmap::{
    mappers::MapperOffset, BarcodeIndexCounter, BusCounter, Counter, Mapper, PairedReader,
    ProbeBarcodeIndexCounter, ProbeBusCounter,
};

use crate::progress::ProgressBar;

pub fn map_pairs<M>(
    mut reader: PairedReader,
    target_mapper: &M,
    target_offset: Option<MapperOffset>,
    barcode_size: usize,
    umi_size: usize,
) -> Result<BarcodeIndexCounter>
where
    M: Mapper,
{
    let mut counter = BusCounter::default();
    let mut pbar = ProgressBar::default();
    while let Some(pair) = reader.next() {
        let pair = pair?;
        let bus = pair.as_bus(barcode_size, umi_size);
        if let Ok(index) = target_mapper.map(&bus.seq, target_offset) {
            counter.increment(&bus, index);
        }
        pbar.tick();
    }
    pbar.finish();
    Ok(counter.dedup_umi())
}

pub fn map_probed_pairs<Mt, Mp>(
    mut reader: PairedReader,
    target_mapper: &Mt,
    probe_mapper: &Mp,
    target_offset: Option<MapperOffset>,
    probe_offset: Option<MapperOffset>,
    barcode_size: usize,
    umi_size: usize,
) -> Result<ProbeBarcodeIndexCounter>
where
    Mt: Mapper,
    Mp: Mapper,
{
    let mut counter = ProbeBusCounter::default();
    let mut pbar = ProgressBar::default();
    while let Some(pair) = reader.next() {
        let pair = pair?;
        let bus = pair.as_bus(barcode_size, umi_size);
        let target_index = target_mapper.map(&bus.seq, target_offset);
        let probe_index = probe_mapper.map(&bus.seq, probe_offset);
        match (target_index, probe_index) {
            (Ok(t_idx), Ok(p_idx)) => {
                counter.increment_probe(p_idx, &bus, t_idx);
            }
            _ => {}
        }
        pbar.tick();
    }
    pbar.finish();
    Ok(counter.dedup_umi())
}
