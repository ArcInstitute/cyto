use std::time::Instant;

use anyhow::{Result, bail};
use cyto_cli::ArgsCrispr;
use cyto_core::mappers::{CrisprMapper, MapperOffset, ProbeMapper};
use cyto_io::{write_features, write_statistics};
use log::error;

use super::{
    ibu_map_pairs_binseq, ibu_map_pairs_paraseq, ibu_map_probed_pairs_binseq,
    ibu_map_probed_pairs_paraseq,
    utils::{build_filepath, build_filepaths, delete_empty_path, delete_empty_paths},
};

pub fn probed_bus(args: &ArgsCrispr) -> Result<()> {
    // Load the input readers
    let paired_readers = args.input.to_fx_readers()?;

    let start_time = Instant::now();

    // Load the target mapper
    let target_mapper =
        CrisprMapper::from_tsv_arc(&args.crispr.guides_filepath, args.map.exact_matching)?;
    let probe_mapper = ProbeMapper::from_tsv_arc(
        args.probe.probes_filepath.as_ref().unwrap(), // already checked
        args.map.exact_matching,
    )?;

    // Define the offsets for the target and probe mappers
    if args.crispr.lookback > args.crispr.offset {
        error!(
            "Lookback ({}) cannot be greater than offset ({})",
            args.crispr.lookback, args.crispr.offset
        );
        bail!(
            "Lookback ({}) cannot be greater than offset ({})",
            args.crispr.lookback,
            args.crispr.offset
        )
    }
    let target_offset = MapperOffset::RightOf(args.crispr.offset);
    let probe_offset = MapperOffset::LeftOf(args.crispr.offset - args.crispr.lookback);

    // Define the file path for each probe
    let filepaths = build_filepaths(&args.output.outdir, &probe_mapper)?;

    // Write the features to the output file
    write_features(&args.output.outdir, target_mapper.as_ref())?;

    // map the reads and write the results to the probe files
    let statistics = ibu_map_probed_pairs_paraseq(
        paired_readers,
        &filepaths,
        target_mapper,
        probe_mapper,
        Some(target_offset),
        Some(probe_offset),
        args.geometry.into(),
        args.runtime.num_threads(),
        args.map.exact_matching,
        args.map.adjustment(),
        start_time,
    )?;

    // Delete the probe files if there are no mapped reads
    delete_empty_paths(&filepaths)?;

    write_statistics(&args.output.outdir, &statistics)?;
    Ok(())
}

pub fn bus(args: &ArgsCrispr) -> Result<()> {
    // Load the input readers
    let paired_readers = args.input.to_fx_readers()?;
    let start_time = Instant::now();
    let target_mapper =
        CrisprMapper::from_tsv_arc(&args.crispr.guides_filepath, args.map.exact_matching)?;
    let target_offset = MapperOffset::RightOf(args.crispr.offset);

    // Define the file path for the output file
    let output_filepath = build_filepath(&args.output.outdir, None);

    // Write the features to the output file
    write_features(&args.output.outdir, target_mapper.as_ref())?;

    // map the reads and write the results to the output file
    let statistics = ibu_map_pairs_paraseq(
        paired_readers,
        &output_filepath,
        target_mapper,
        Some(target_offset),
        args.geometry.into(),
        args.runtime.num_threads(),
        args.map.exact_matching,
        args.map.adjustment(),
        start_time,
    )?;

    // Delete the output file if there are no mapped reads
    delete_empty_path(&output_filepath)?;

    write_statistics(&args.output.outdir, &statistics)?;
    Ok(())
}

fn bus_binseq(args: &ArgsCrispr) -> Result<()> {
    let readers = args.input.to_binseq_readers()?;
    let start_time = Instant::now();
    let target_mapper =
        CrisprMapper::from_tsv_arc(&args.crispr.guides_filepath, args.map.exact_matching)?;
    let target_offset = MapperOffset::RightOf(args.crispr.offset);

    // Define the file path for the output file
    let output_filepath = build_filepath(&args.output.outdir, None);

    // Write the features to the output file
    write_features(&args.output.outdir, target_mapper.as_ref())?;

    // Open a file handle for the output file
    let statistics = ibu_map_pairs_binseq(
        readers,
        &output_filepath,
        target_mapper,
        Some(target_offset),
        args.geometry.into(),
        args.runtime.num_threads(),
        args.map.exact_matching,
        args.map.adjustment(),
        start_time,
    )?;

    write_statistics(&args.output.outdir, &statistics)?;
    Ok(())
}

pub fn probed_bus_binseq(args: &ArgsCrispr) -> Result<()> {
    let readers = args.input.to_binseq_readers()?;
    let start_time = Instant::now();
    let target_mapper =
        CrisprMapper::from_tsv_arc(&args.crispr.guides_filepath, args.map.exact_matching)?;
    let probe_mapper = ProbeMapper::from_tsv_arc(
        args.probe.probes_filepath.as_ref().unwrap(), // already checked
        args.map.exact_matching,
    )?;

    if args.crispr.lookback >= args.crispr.offset {
        error!(
            "Lookback ({}) cannot be greater or equal to offset ({})",
            args.crispr.lookback, args.crispr.offset
        );
        bail!(
            "Lookback ({}) cannot be greater or equal to offset ({})",
            args.crispr.lookback,
            args.crispr.offset
        )
    }
    let target_offset = MapperOffset::RightOf(args.crispr.offset);
    let probe_offset = MapperOffset::LeftOf(args.crispr.offset - args.crispr.lookback);

    let filepaths = build_filepaths(&args.output.outdir, &probe_mapper)?;

    write_features(&args.output.outdir, target_mapper.as_ref())?;

    let statistics = ibu_map_probed_pairs_binseq(
        readers,
        &filepaths,
        target_mapper,
        probe_mapper,
        Some(target_offset),
        Some(probe_offset),
        args.geometry.into(),
        args.runtime.num_threads(),
        args.map.exact_matching,
        args.map.adjustment(),
        start_time,
    )?;

    delete_empty_paths(&filepaths)?;

    write_statistics(&args.output.outdir, &statistics)?;
    Ok(())
}

pub fn run(args: &ArgsCrispr) -> Result<()> {
    if args.probe.probes_filepath.is_some() {
        if args.input.is_binseq() {
            probed_bus_binseq(args)
        } else {
            probed_bus(args)
        }
    } else if args.input.is_binseq() {
        bus_binseq(args)
    } else {
        bus(args)
    }
}
