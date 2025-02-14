use crate::{
    cli::ArgsCrispr,
    io::{write_features, write_statistics},
};
use anyhow::Result;
use cyto::{
    libraries::{CrisprLibrary, ProbeLibrary},
    mappers::MapperOffset,
};

use super::{
    ibu_map_pairs_paraseq, ibu_map_probed_pairs_paraseq,
    utils::{build_filepath, build_filepaths, delete_empty_path, delete_empty_paths},
};

#[cfg(feature = "binseq")]
use super::ibu_map_pairs_binseq;

pub fn probed_bus(args: ArgsCrispr) -> Result<()> {
    // Load the input readers
    let (r1, r2) = args.input.to_readers()?;

    // Load the target mapper
    let target_library = CrisprLibrary::from_tsv(args.crispr.guides_filepath.into())?;
    let target_mapper = if args.crispr.exact_matching {
        target_library.into_mapper()
    } else {
        target_library.into_corrected_mapper()
    }?;

    // Load the probe mapper
    let probe_library = ProbeLibrary::from_tsv(args.probe.probes_filepath.unwrap().into())?;
    let probe_mapper = if args.crispr.exact_matching {
        probe_library.into_mapper()
    } else {
        probe_library.into_corrected_mapper()
    }?;

    // Define the offsets for the target and probe mappers
    let target_offset = MapperOffset::RightOf(args.crispr.offset);
    let probe_offset = MapperOffset::LeftOf(args.crispr.offset);

    // Define the file path for each probe
    let filepaths = build_filepaths(&args.output.prefix, &probe_mapper)?;

    // Write the features to the output file
    write_features(&args.output, &target_mapper)?;

    // map the reads and write the results to the probe files
    let statistics = ibu_map_probed_pairs_paraseq(
        r1,
        r2,
        &filepaths,
        target_mapper,
        probe_mapper,
        Some(target_offset),
        Some(probe_offset),
        args.geometry.into(),
        args.runtime.num_threads(),
        args.crispr.exact_matching,
    )?;

    // Delete the probe files if there are no mapped reads
    delete_empty_paths(&filepaths)?;

    write_statistics(&args.output, &statistics)?;
    Ok(())
}

pub fn bus(args: ArgsCrispr) -> Result<()> {
    // Load the input readers
    let (r1, r2) = args.input.to_readers()?;

    let target_library = CrisprLibrary::from_tsv(args.crispr.guides_filepath.into())?;
    let target_mapper = if args.crispr.exact_matching {
        target_library.into_mapper()
    } else {
        target_library.into_corrected_mapper()
    }?;
    let target_offset = MapperOffset::RightOf(args.crispr.offset);

    // Define the file path for the output file
    let output_filepath = build_filepath(&args.output.prefix, None);

    // Write the features to the output file
    write_features(&args.output, &target_mapper)?;

    // map the reads and write the results to the output file
    let statistics = ibu_map_pairs_paraseq(
        r1,
        r2,
        &output_filepath,
        target_mapper,
        Some(target_offset),
        args.geometry.into(),
        args.runtime.num_threads(),
        args.crispr.exact_matching,
    )?;

    // Delete the output file if there are no mapped reads
    delete_empty_path(&output_filepath)?;

    write_statistics(&args.output, &statistics)?;
    Ok(())
}

#[cfg(feature = "binseq")]
fn bus_binseq(args: ArgsCrispr) -> Result<()> {
    let reader = args.binseq.into_reader()?;
    let target_library = CrisprLibrary::from_tsv(args.crispr.guides_filepath.into())?;
    let target_mapper = if args.crispr.exact_matching {
        target_library.into_mapper()
    } else {
        target_library.into_corrected_mapper()
    }?;

    let target_offset = MapperOffset::RightOf(args.crispr.offset);

    // Define the file path for the output file
    let output_filepath = build_filepath(&args.output.prefix, None);

    // Write the features to the output file
    write_features(&args.output, &target_mapper)?;

    // Open a file handle for the output file
    let statistics = ibu_map_pairs_binseq(
        reader,
        output_filepath,
        target_mapper,
        Some(target_offset),
        args.geometry.into(),
        args.binseq.num_threads(),
        args.crispr.exact_matching,
    )?;

    write_statistics(&args.output, &statistics)?;
    Ok(())
}

pub fn run(args: ArgsCrispr) -> Result<()> {
    if args.probe.probes_filepath.is_some() {
        probed_bus(args)
    } else {
        #[cfg(feature = "binseq")]
        if args.binseq.input.is_some() {
            return bus_binseq(args);
        }
        bus(args)
    }
}
