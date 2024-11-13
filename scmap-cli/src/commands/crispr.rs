use crate::cli::ArgsCrispr;
use anyhow::Result;
use scmap::{
    crispr::Library as CrisprLibrary, io::write_sparse_mtx, probe::Library as ProbeLibrary,
    BusCounter, PairedReader, ProbeBusCounter,
};
use std::{fs::File, io::BufWriter};

pub fn probed_bus(args: ArgsCrispr) -> Result<()> {
    let guide_mapper =
        CrisprLibrary::from_tsv(args.crispr.guides_filepath.into())?.into_mapper()?;
    let probe_mapper =
        ProbeLibrary::from_tsv(args.probe.probes_filepath.unwrap().into())?.into_mapper()?;
    let mut counter = ProbeBusCounter::default();

    for pair in PairedReader::new(&args.input.r1, &args.input.r2)? {
        let bus = pair.as_bus(args.geometry.barcode, args.geometry.umi);
        let guide_index = guide_mapper.map(&bus.seq, args.crispr.offset);
        let probe = probe_mapper.map(&bus.seq, args.crispr.offset);
        match (guide_index, probe) {
            (Some(g_idx), Some(p_idx)) => {
                counter.increment(p_idx, &bus, g_idx);
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

pub fn bus(args: ArgsCrispr) -> Result<()> {
    let guide_mapper =
        CrisprLibrary::from_tsv(args.crispr.guides_filepath.into())?.into_mapper()?;
    let mut counter = BusCounter::default();

    for pair in PairedReader::new(&args.input.r1, &args.input.r2)? {
        let bus = pair.as_bus(args.geometry.barcode, args.geometry.umi);
        if let Some(guide_index) = guide_mapper.map(&bus.seq, args.crispr.offset) {
            counter.increment(&bus, guide_index);
        }
    }

    let output_path = format!("{}.mtx", &args.output.prefix);
    let mut output_handle = File::create(output_path).map(BufWriter::new)?;
    write_sparse_mtx(&mut output_handle, &counter, args.output.with_header)
}

pub fn run(args: ArgsCrispr) -> Result<()> {
    if args.probe.probes_filepath.is_some() {
        probed_bus(args)
    } else {
        bus(args)
    }
}
