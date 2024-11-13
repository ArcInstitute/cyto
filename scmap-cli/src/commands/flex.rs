use crate::{
    cli::ArgsFlex,
    io::{write_bus_matrix, write_probe_matrices},
};
use anyhow::Result;
use scmap::{
    libraries::{FlexLibrary, ProbeLibrary},
    BusCounter, PairedReader, ProbeBusCounter,
};

fn probed_bus(args: ArgsFlex) -> Result<()> {
    let flex_mapper = FlexLibrary::from_tsv(args.flex.flex_filepath.into())?.into_mapper()?;
    let probe_mapper =
        ProbeLibrary::from_tsv(args.probe.probes_filepath.unwrap().into())?.into_mapper()?;
    let mut counter = ProbeBusCounter::default();

    // The expected start position of the probe sequence in the bus sequence
    let probe_offset = flex_mapper.get_sequence_size() + args.flex.spacer;

    for pair in PairedReader::new(&args.input.r1, &args.input.r2)? {
        let bus = pair.as_bus(args.geometry.barcode, args.geometry.umi);
        let flex_index = flex_mapper.map(&bus.seq);
        let probe = probe_mapper.map_right(&bus.seq, probe_offset);
        match (flex_index, probe) {
            (Some(f_idx), Some(p_idx)) => {
                counter.increment(p_idx, &bus, f_idx);
            }
            _ => {}
        }
    }
    let counts = counter.dedup_umi();
    write_probe_matrices(&args.output, &probe_mapper, &counts)
}

fn bus(args: ArgsFlex) -> Result<()> {
    let flex_mapper = FlexLibrary::from_tsv(args.flex.flex_filepath.into())?.into_mapper()?;
    let mut counter = BusCounter::default();

    for pair in PairedReader::new(&args.input.r1, &args.input.r2)? {
        let bus = pair.as_bus(args.geometry.barcode, args.geometry.umi);
        if let Some(flex_index) = flex_mapper.map(&bus.seq) {
            counter.increment(&bus, flex_index);
        }
    }
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
