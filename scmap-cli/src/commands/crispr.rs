use crate::{
    cli::ArgsCrispr,
    io::{write_bus_matrix, write_probe_matrices},
};
use anyhow::Result;
use scmap::{
    libraries::{CrisprLibrary, ProbeLibrary},
    mappers::{Mapper, MapperOffset},
    BusCounter, Counter, PairedReader, ProbeBusCounter,
};

pub fn probed_bus(args: ArgsCrispr) -> Result<()> {
    let guide_mapper =
        CrisprLibrary::from_tsv(args.crispr.guides_filepath.into())?.into_mapper()?;
    let probe_mapper =
        ProbeLibrary::from_tsv(args.probe.probes_filepath.unwrap().into())?.into_mapper()?;
    let mut counter = ProbeBusCounter::default();
    let guide_offset = MapperOffset::RightOf(args.crispr.offset);
    let probe_offset = MapperOffset::LeftOf(args.crispr.offset);

    for pair in PairedReader::new(&args.input.r1, &args.input.r2)? {
        let bus = pair.as_bus(args.geometry.barcode, args.geometry.umi);
        let guide_index = guide_mapper.map(&bus.seq, Some(guide_offset));
        let probe = probe_mapper.map(&bus.seq, Some(probe_offset));
        match (guide_index, probe) {
            (Some(g_idx), Some(p_idx)) => {
                counter.increment(p_idx, &bus, g_idx);
            }
            _ => {}
        }
    }

    let counts = counter.dedup_umi();
    write_probe_matrices(&args.output, &probe_mapper, &counts)
}

pub fn bus(args: ArgsCrispr) -> Result<()> {
    let guide_mapper =
        CrisprLibrary::from_tsv(args.crispr.guides_filepath.into())?.into_mapper()?;
    let mut counter = BusCounter::default();
    let guide_offset = MapperOffset::RightOf(args.crispr.offset);

    for pair in PairedReader::new(&args.input.r1, &args.input.r2)? {
        let bus = pair.as_bus(args.geometry.barcode, args.geometry.umi);
        if let Some(guide_index) = guide_mapper.map(&bus.seq, Some(guide_offset)) {
            counter.increment(&bus, guide_index);
        }
    }

    let counts = counter.dedup_umi();
    write_bus_matrix(&args.output, &counts)
}

pub fn run(args: ArgsCrispr) -> Result<()> {
    if args.probe.probes_filepath.is_some() {
        probed_bus(args)
    } else {
        bus(args)
    }
}
