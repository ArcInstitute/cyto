use super::{map_pairs, map_probed_pairs};
use crate::{
    cli::ArgsCrispr,
    io::{write_bus_matrix, write_probe_matrices},
};
use anyhow::Result;
use scmap::{
    libraries::{CrisprLibrary, ProbeLibrary},
    mappers::MapperOffset,
    BusCounter, PairedReader, ProbeBusCounter,
};

pub fn probed_bus(args: ArgsCrispr) -> Result<()> {
    let reader = PairedReader::new(&args.input.r1, &args.input.r2)?;
    let target_mapper =
        CrisprLibrary::from_tsv(args.crispr.guides_filepath.into())?.into_mapper()?;
    let probe_mapper =
        ProbeLibrary::from_tsv(args.probe.probes_filepath.unwrap().into())?.into_mapper()?;
    let mut counter = ProbeBusCounter::default();
    let target_offset = MapperOffset::RightOf(args.crispr.offset);
    let probe_offset = MapperOffset::LeftOf(args.crispr.offset);

    map_probed_pairs(
        reader,
        &mut counter,
        &target_mapper,
        &probe_mapper,
        Some(target_offset),
        Some(probe_offset),
        args.geometry.barcode,
        args.geometry.umi,
    );

    let counts = counter.dedup_umi();
    write_probe_matrices(&args.output, &probe_mapper, &counts)
}

pub fn bus(args: ArgsCrispr) -> Result<()> {
    let reader = PairedReader::new(&args.input.r1, &args.input.r2)?;
    let target_mapper =
        CrisprLibrary::from_tsv(args.crispr.guides_filepath.into())?.into_mapper()?;
    let mut counter = BusCounter::default();
    let target_offset = MapperOffset::RightOf(args.crispr.offset);

    map_pairs(
        reader,
        &mut counter,
        &target_mapper,
        Some(target_offset),
        args.geometry.barcode,
        args.geometry.umi,
    );
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
