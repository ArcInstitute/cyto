use std::time::Instant;

use anyhow::Result;
use cyto_cli::ArgsGex;
use cyto_core::mappers::{GexMapper, MapperOffset, ProbeMapper};
use cyto_io::{write_features, write_statistics};

use super::{
    ibu_map_pairs_binseq, ibu_map_pairs_paraseq, ibu_map_probed_pairs_binseq,
    ibu_map_probed_pairs_paraseq,
    utils::{build_filepath, build_filepaths, delete_empty_path, delete_empty_paths},
};

fn probed_bus(args: &ArgsGex) -> Result<()> {
    let paired_readers = args.input.to_fx_readers()?;
    let start_time = Instant::now();

    // Load the target library
    let target_mapper = GexMapper::from_tsv_arc(&args.gex.gex_filepath)?;

    // Load the probe library
    let probe_mapper = ProbeMapper::from_tsv_arc(
        args.probe.probes_filepath.as_ref().unwrap(), // already checked
        args.map.exact_matching,
    )?;

    // The expected start position of the probe sequence in the bus sequence
    let probe_offset = MapperOffset::RightOf(target_mapper.get_sequence_size() + args.gex.spacer);

    // Define the file path for each probe
    let filepaths = build_filepaths(&args.output.outdir, &probe_mapper)?;

    // Write the features to the output file
    write_features(&args.output.outdir, target_mapper.as_ref())?;

    let statistics = ibu_map_probed_pairs_paraseq(
        paired_readers,
        &filepaths,
        target_mapper,
        probe_mapper,
        None,
        Some(probe_offset),
        args.geometry.into(),
        args.runtime.num_threads(),
        args.map.exact_matching,
        args.map.adjustment,
        start_time,
    )?;

    // Delete the probe files if there are no mapped reads
    delete_empty_paths(&filepaths)?;

    write_statistics(&args.output.outdir, &statistics)?;
    Ok(())
}

fn bus(args: &ArgsGex) -> Result<()> {
    // Load the input files
    let paired_readers = args.input.to_fx_readers()?;

    let start_time = Instant::now();

    // Load the target library
    let target_mapper = GexMapper::from_tsv_arc(&args.gex.gex_filepath)?;

    // Define the file path for the output file
    let output_filepath = build_filepath(&args.output.outdir, None);

    // Write the features to the output file
    write_features(&args.output.outdir, target_mapper.as_ref())?;

    let statistics = ibu_map_pairs_paraseq(
        paired_readers,
        &output_filepath,
        target_mapper,
        None,
        args.geometry.into(),
        args.runtime.num_threads(),
        args.map.exact_matching,
        args.map.adjustment,
        start_time,
    )?;

    // Delete the output file if there are no mapped reads
    delete_empty_path(&output_filepath)?;

    // Write the statistics to the output file
    write_statistics(&args.output.outdir, &statistics)?;
    Ok(())
}

fn bus_binseq(args: &ArgsGex) -> Result<()> {
    let readers = args.input.to_binseq_readers()?;
    let start_time = Instant::now();

    // Load the target library
    let target_mapper = GexMapper::from_tsv_arc(&args.gex.gex_filepath)?;

    // Define the file path for the output file
    let output_filepath = build_filepath(&args.output.outdir, None);

    // Write the features to the output file
    write_features(&args.output.outdir, target_mapper.as_ref())?;

    // Open a file handle for the output file
    let statistics = ibu_map_pairs_binseq(
        readers,
        &output_filepath,
        target_mapper,
        None,
        args.geometry.into(),
        args.runtime.num_threads(),
        args.map.exact_matching,
        args.map.adjustment,
        start_time,
    )?;

    write_statistics(&args.output.outdir, &statistics)?;
    Ok(())
}

pub fn probed_bus_binseq(args: &ArgsGex) -> Result<()> {
    // load in multiple binseq readers
    let readers = args.input.to_binseq_readers()?;

    let start_time = Instant::now();

    // Load the target library
    let target_mapper = GexMapper::from_tsv_arc(&args.gex.gex_filepath)?;

    // Load the probe library
    let probe_mapper = ProbeMapper::from_tsv_arc(
        args.probe.probes_filepath.as_ref().unwrap(), // already checked
        args.map.exact_matching,
    )?;

    // The expected start position of the probe sequence in the bus sequence
    let probe_offset = MapperOffset::RightOf(target_mapper.get_sequence_size() + args.gex.spacer);

    // Define the file path for each probe
    let filepaths = build_filepaths(&args.output.outdir, &probe_mapper)?;

    // Write the features to the output file
    write_features(&args.output.outdir, target_mapper.as_ref())?;

    let statistics = ibu_map_probed_pairs_binseq(
        readers,
        &filepaths,
        target_mapper,
        probe_mapper,
        None,
        Some(probe_offset),
        args.geometry.into(),
        args.runtime.num_threads(),
        args.map.exact_matching,
        args.map.adjustment,
        start_time,
    )?;

    // Delete the probe files if there are no mapped reads
    delete_empty_paths(&filepaths)?;

    write_statistics(&args.output.outdir, &statistics)?;

    Ok(())
}

pub fn run(args: &ArgsGex) -> Result<()> {
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
