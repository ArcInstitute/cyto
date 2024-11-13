use crate::{
    cli::ArgsFlex,
    io::{write_bus_matrix, write_probe_matrices},
};
use anyhow::Result;
use scmap::{
    libraries::{FlexLibrary, ProbeLibrary},
    mappers::MapperOffset,
    BusCounter, PairedReader, ProbeBusCounter,
};

use super::{map_pairs, map_probed_pairs};

fn probed_bus(args: ArgsFlex) -> Result<()> {
    let reader = PairedReader::new(&args.input.r1, &args.input.r2)?;
    let target_mapper = FlexLibrary::from_tsv(args.flex.flex_filepath.into())?.into_mapper()?;
    let probe_mapper =
        ProbeLibrary::from_tsv(args.probe.probes_filepath.unwrap().into())?.into_mapper()?;
    let mut counter = ProbeBusCounter::default();

    // The expected start position of the probe sequence in the bus sequence
    let probe_offset = MapperOffset::RightOf(target_mapper.get_sequence_size() + args.flex.spacer);

    map_probed_pairs(
        reader,
        &mut counter,
        &target_mapper,
        &probe_mapper,
        None,
        Some(probe_offset),
        args.geometry.barcode,
        args.geometry.umi,
    );
    let counts = counter.dedup_umi();
    write_probe_matrices(&args.output, &probe_mapper, &counts)
}

fn bus(args: ArgsFlex) -> Result<()> {
    let reader = PairedReader::new(&args.input.r1, &args.input.r2)?;
    let target_mapper = FlexLibrary::from_tsv(args.flex.flex_filepath.into())?.into_mapper()?;
    let mut counter = BusCounter::default();
    map_pairs(
        reader,
        &mut counter,
        &target_mapper,
        None,
        args.geometry.barcode,
        args.geometry.umi,
    );
    let counts = counter.dedup_umi();
    write_bus_matrix(&args.output, &counts)
}

pub fn run(args: ArgsFlex) -> Result<()> {
    if args.probe.probes_filepath.is_some() {
        probed_bus(args)
    } else {
        bus(args)
    }
}
