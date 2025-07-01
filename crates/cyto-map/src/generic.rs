use std::time::Instant;

use anyhow::Result;
use cyto_cli::map::ArgsGeneric;
use cyto_core::mappers::GenericMapper;
use cyto_io::{write_features, write_statistics};

use super::{
    ibu_map_pairs_binseq,
    implementor::ibu_map_pairs_paraseq,
    utils::{build_filepath, delete_empty_path, find_offset_paraseq},
};

fn bus(args: &ArgsGeneric) -> Result<()> {
    // Load the input files
    let (r1, mut r2) = args.input.to_readers()?;
    let start_time = Instant::now();

    // Load the target library
    let target_mapper =
        GenericMapper::from_tsv_arc(&args.generic.generic_filepath, args.map.exact_matching)?;

    // if not offset is provided, find the best fit
    let offset = if let Some(offset) = args.generic.offset() {
        offset
    } else {
        find_offset_paraseq(&mut r2, target_mapper.as_ref())?
    };

    // Define the file path for the output file
    let output_filepath = build_filepath(&args.output.prefix, None);

    // Write the features to the output file
    write_features(&args.output.prefix, target_mapper.as_ref())?;

    let statistics = ibu_map_pairs_paraseq(
        r1,
        r2,
        &output_filepath,
        target_mapper,
        Some(offset),
        args.geometry.into(),
        args.runtime.num_threads(),
        args.map.exact_matching,
        args.map.adjustment,
        start_time,
    )?;

    // Delete the output file if there are no mapped reads
    delete_empty_path(&output_filepath)?;

    // Write the statistics to the output file
    write_statistics(&args.output.prefix, &statistics)?;
    Ok(())
}

fn bus_binseq(args: &ArgsGeneric) -> Result<()> {
    use super::utils::find_offset_binseq;

    let reader = args.binseq.into_reader()?;
    let start_time = Instant::now();

    // Load the target library
    let target_mapper =
        GenericMapper::from_tsv_arc(&args.generic.generic_filepath, args.map.exact_matching)?;

    // Calculate the offset if not provided
    let offset = if let Some(offset) = args.generic.offset() {
        offset
    } else {
        find_offset_binseq(args.binseq.path()?, target_mapper.clone(), 1024)?
    };

    // Define the file path for the output file
    let output_filepath = build_filepath(&args.output.prefix, None);

    // Write the features to the output file
    write_features(&args.output.prefix, target_mapper.as_ref())?;

    // Open a file handle for the output file
    let statistics = ibu_map_pairs_binseq(
        reader,
        &output_filepath,
        target_mapper,
        Some(offset),
        args.geometry.into(),
        args.runtime.num_threads(),
        args.map.exact_matching,
        args.map.adjustment,
        start_time,
    )?;

    write_statistics(&args.output.prefix, &statistics)?;
    Ok(())
}

pub fn run(args: &ArgsGeneric) -> Result<()> {
    if args.binseq.input.is_some() {
        return bus_binseq(args);
    }
    bus(args)
}
