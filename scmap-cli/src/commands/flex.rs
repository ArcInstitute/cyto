use crate::cli::ArgsFlex;
use anyhow::Result;
use scmap::{
    flex::Library as FlexLibrary, io::write_sparse_mtx, probe::Library as ProbeLibrary, BusCounter,
    PairedReader, ProbeBusCounter,
};
use std::{fs::File, io::BufWriter};

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

    for p_idx in counter.iter_probes() {
        let probe_alias = probe_mapper.get_alias(*p_idx).unwrap().name_str()?;
        let output_path = format!("{}.{}.mtx", &args.output.prefix, probe_alias);
        let mut output_handle = File::create(output_path).map(BufWriter::new)?;
        let bus_counter = counter.get_probe_counter(*p_idx).unwrap();
        write_sparse_mtx(&mut output_handle, bus_counter, args.output.with_header)?;
    }

    Ok(())
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

    let output_path = format!("{}.mtx", &args.output.prefix);
    let mut output_handle = File::create(output_path).map(BufWriter::new)?;
    write_sparse_mtx(&mut output_handle, &counter, args.output.with_header)
}

pub fn run(args: ArgsFlex) -> Result<()> {
    if args.probe.probes_filepath.is_some() {
        probed_bus(args)
    } else {
        bus(args)
    }
}
