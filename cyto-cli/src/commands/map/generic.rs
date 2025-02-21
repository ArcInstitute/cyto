use std::time::Instant;

use anyhow::Result;
use cyto::mappers::GenericMapper;

use crate::{
    cli::map::ArgsGeneric,
    io::{write_features, write_statistics},
};

use super::{
    implementor::ibu_map_pairs_paraseq,
    utils::{build_filepath, delete_empty_path},
};

#[cfg(feature = "binseq")]
use super::ibu_map_pairs_binseq;

fn bus(args: ArgsGeneric) -> Result<()> {
    // Load the input files
    let (r1, r2) = args.input.to_readers()?;
    let offset = args.generic.offset();

    let start_time = Instant::now();

    // Load the target library
    let target_mapper =
        GenericMapper::from_tsv_arc(&args.generic.generic_filepath, args.map.exact_matching)?;

    // Define the file path for the output file
    let output_filepath = build_filepath(&args.output.prefix, None);

    // Write the features to the output file
    write_features(&args.output, target_mapper.as_ref())?;

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
    write_statistics(&args.output, &statistics)?;
    Ok(())
}

#[cfg(feature = "binseq")]
fn bus_binseq(args: ArgsGeneric) -> Result<()> {
    let reader = args.binseq.into_reader()?;
    let offset = args.generic.offset();

    let start_time = Instant::now();

    // Load the target library
    let target_mapper =
        GenericMapper::from_tsv_arc(&args.generic.generic_filepath, args.map.exact_matching)?;

    // Define the file path for the output file
    let output_filepath = build_filepath(&args.output.prefix, None);

    // Write the features to the output file
    write_features(&args.output, target_mapper.as_ref())?;

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

    write_statistics(&args.output, &statistics)?;
    Ok(())
}

pub fn run(args: ArgsGeneric) -> Result<()> {
    #[cfg(feature = "binseq")]
    if args.binseq.input.is_some() {
        return bus_binseq(args);
    }
    bus(args)
}
