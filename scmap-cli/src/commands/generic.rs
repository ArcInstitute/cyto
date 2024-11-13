use scmap::{mappers::MapperOffset, Counter, Mapper, PairedReader};

pub fn map_pairs<M, C>(
    reader: PairedReader,
    counter: &mut C,
    target_mapper: &M,
    target_offset: Option<MapperOffset>,
    barcode_size: usize,
    umi_size: usize,
) where
    M: Mapper,
    C: Counter,
{
    for pair in reader {
        let bus = pair.as_bus(barcode_size, umi_size);
        if let Some(index) = target_mapper.map(&bus.seq, target_offset) {
            counter.increment(&bus, index);
        }
    }
}

pub fn map_probed_pairs<Mt, Mp, C>(
    reader: PairedReader,
    counter: &mut C,
    target_mapper: &Mt,
    probe_mapper: &Mp,
    target_offset: Option<MapperOffset>,
    probe_offset: Option<MapperOffset>,
    barcode_size: usize,
    umi_size: usize,
) where
    Mt: Mapper,
    Mp: Mapper,
    C: Counter,
{
    for pair in reader {
        let bus = pair.as_bus(barcode_size, umi_size);
        let target_index = target_mapper.map(&bus.seq, target_offset);
        let probe_index = probe_mapper.map(&bus.seq, probe_offset);
        match (target_index, probe_index) {
            (Some(t_idx), Some(p_idx)) => {
                counter.increment_probe(p_idx, &bus, t_idx);
            }
            _ => {}
        }
    }
}
