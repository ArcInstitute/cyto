use std::sync::Arc;
use std::time::Instant;

use crate::{
    cli::ArgsFlex,
    io::{write_features, write_statistics},
};
use anyhow::Result;
use cyto::{
    libraries::{FlexLibrary, ProbeLibrary},
    mappers::MapperOffset,
};

use super::{
    ibu_map_pairs_paraseq, ibu_map_probed_pairs_paraseq,
    utils::{build_filepath, build_filepaths, delete_empty_path, delete_empty_paths},
};

#[cfg(feature = "binseq")]
use super::{ibu_map_pairs_binseq, ibu_map_probed_pairs_binseq};

fn probed_bus(args: ArgsFlex) -> Result<()> {
    let (r1, r2) = args.input.to_readers()?;

    let start_time = Instant::now();

    // Load the target library
    let target_library = FlexLibrary::from_tsv(args.flex.flex_filepath.into())?;
    let target_mapper = if args.map.exact_matching {
        target_library.into_mapper()
    } else {
        target_library.into_corrected_mapper()
    }?;

    // Load the probe library
    let probe_library = ProbeLibrary::from_tsv(args.probe.probes_filepath.unwrap().into())?;
    let probe_mapper = if args.map.exact_matching {
        probe_library.into_mapper()
    } else {
        probe_library.into_corrected_mapper()
    }?;

    // The expected start position of the probe sequence in the bus sequence
    let probe_offset = MapperOffset::RightOf(target_mapper.get_sequence_size() + args.flex.spacer);

    // Define the file path for each probe
    let filepaths = build_filepaths(&args.output.prefix, &probe_mapper)?;

    // Write the features to the output file
    write_features(&args.output, &target_mapper)?;

    let statistics = ibu_map_probed_pairs_paraseq(
        r1,
        r2,
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

    write_statistics(&args.output, &statistics)?;
    Ok(())
}

fn bus(args: ArgsFlex) -> Result<()> {
    // Load the input files
    let (r1, r2) = args.input.to_readers()?;

    let start_time = Instant::now();

    let target_library = FlexLibrary::from_tsv(args.flex.flex_filepath.into())?;
    let target_mapper = if args.map.exact_matching {
        target_library.into_mapper()
    } else {
        target_library.into_corrected_mapper()
    }?;
    let target_mapper = Arc::new(target_mapper);

    // Define the file path for the output file
    let output_filepath = build_filepath(&args.output.prefix, None);

    // Write the features to the output file
    write_features(&args.output, target_mapper.as_ref())?;

    let statistics = ibu_map_pairs_paraseq(
        r1,
        r2,
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
    write_statistics(&args.output, &statistics)?;
    Ok(())
}

#[cfg(feature = "binseq")]
fn bus_binseq(args: ArgsFlex) -> Result<()> {
    let reader = args.binseq.into_reader()?;
    let start_time = Instant::now();
    let target_library = FlexLibrary::from_tsv(args.flex.flex_filepath.into())?;
    let target_mapper = if args.map.exact_matching {
        target_library.into_mapper()
    } else {
        target_library.into_corrected_mapper()
    }?;
    let target_mapper = Arc::new(target_mapper);

    // Define the file path for the output file
    let output_filepath = build_filepath(&args.output.prefix, None);

    // Write the features to the output file
    write_features(&args.output, target_mapper.as_ref())?;

    // Open a file handle for the output file
    let statistics = ibu_map_pairs_binseq(
        reader,
        &output_filepath,
        target_mapper,
        None,
        args.geometry.into(),
        args.runtime.num_threads(),
        args.map.exact_matching,
        args.map.adjustment,
        start_time,
    )?;

    write_statistics(&args.output, &statistics)?;
    Ok(())
}

#[cfg(feature = "binseq")]
pub fn probed_bus_binseq(args: ArgsFlex) -> Result<()> {
    let reader = args.binseq.into_reader()?;

    let start_time = Instant::now();

    // Load the target library
    let target_library = FlexLibrary::from_tsv(args.flex.flex_filepath.into())?;
    let target_mapper = if args.map.exact_matching {
        target_library.into_mapper()
    } else {
        target_library.into_corrected_mapper()
    }?;

    // Load the probe library
    let probe_library = ProbeLibrary::from_tsv(args.probe.probes_filepath.unwrap().into())?;
    let probe_mapper = if args.map.exact_matching {
        probe_library.into_mapper()
    } else {
        probe_library.into_corrected_mapper()
    }?;

    // The expected start position of the probe sequence in the bus sequence
    let probe_offset = MapperOffset::RightOf(target_mapper.get_sequence_size() + args.flex.spacer);

    // Define the file path for each probe
    let filepaths = build_filepaths(&args.output.prefix, &probe_mapper)?;

    // Write the features to the output file
    write_features(&args.output, &target_mapper)?;

    let statistics = ibu_map_probed_pairs_binseq(
        reader,
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

    write_statistics(&args.output, &statistics)?;

    Ok(())
}

pub fn run(args: ArgsFlex) -> Result<()> {
    if args.probe.probes_filepath.is_some() {
        #[cfg(feature = "binseq")]
        if args.binseq.input.is_some() {
            return probed_bus_binseq(args);
        }
        probed_bus(args)
    } else {
        #[cfg(feature = "binseq")]
        if args.binseq.input.is_some() {
            return bus_binseq(args);
        }
        bus(args)
    }
}
