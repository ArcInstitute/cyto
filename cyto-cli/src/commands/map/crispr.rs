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
    ibu_map_pairs, ibu_map_probed_pairs,
    utils::{
        build_filepath, build_filepaths, delete_empty_path, delete_empty_paths, open_handle,
        open_handles,
    },
};

#[cfg(feature = "binseq")]
use super::ibu_map_pairs_binseq;

pub fn probed_bus(args: ArgsCrispr) -> Result<()> {
    let readers = args.input.into_readers()?;
    let target_mapper =
        CrisprLibrary::from_tsv(args.crispr.guides_filepath.into())?.into_mapper()?;
    let probe_mapper =
        ProbeLibrary::from_tsv(args.probe.probes_filepath.unwrap().into())?.into_mapper()?;
    let target_offset = MapperOffset::RightOf(args.crispr.offset);
    let probe_offset = MapperOffset::LeftOf(args.crispr.offset);

    // Define the file path for each probe
    let filepaths = build_filepaths(&args.output.prefix, &probe_mapper)?;

    // Open a file handle for each handle
    let mut probe_writers = open_handles(&filepaths)?;

    // map the reads and write the results to the probe files
    let statistics = ibu_map_probed_pairs(
        readers,
        &mut probe_writers,
        &target_mapper,
        &probe_mapper,
        Some(target_offset),
        Some(probe_offset),
        args.geometry.into(),
    )?;

    // Delete the probe files if there are no mapped reads
    delete_empty_paths(&filepaths)?;

    write_statistics(&args.output, &statistics)?;
    write_features(&args.output, &target_mapper)?;
    Ok(())
}

pub fn bus(args: ArgsCrispr) -> Result<()> {
    let readers = args.input.into_readers()?;
    let target_mapper =
        CrisprLibrary::from_tsv(args.crispr.guides_filepath.into())?.into_mapper()?;
    let target_offset = MapperOffset::RightOf(args.crispr.offset);

    // Define the file path for the output file
    let output_filepath = build_filepath(&args.output.prefix, None);

    // Open a file handle for the output file
    let mut handle = open_handle(&output_filepath)?;

    // map the reads and write the results to the output file
    let statistics = ibu_map_pairs(
        readers,
        &mut handle,
        &target_mapper,
        Some(target_offset),
        args.geometry.into(),
    )?;

    // Delete the output file if there are no mapped reads
    delete_empty_path(&output_filepath)?;

    write_statistics(&args.output, &statistics)?;
    write_features(&args.output, &target_mapper)?;
    Ok(())
}

#[cfg(feature = "binseq")]
fn bus_binseq(args: ArgsCrispr) -> Result<()> {
    let reader = args.binseq.into_reader()?;
    let target_mapper =
        CrisprLibrary::from_tsv(args.crispr.guides_filepath.into())?.into_mapper()?;
    let target_offset = MapperOffset::RightOf(args.crispr.offset);

    // Define the file path for the output file
    let output_filepath = build_filepath(&args.output.prefix, None);

    // Open a file handle for the output file
    let statistics = ibu_map_pairs_binseq(
        reader,
        output_filepath,
        target_mapper,
        Some(target_offset),
        args.geometry.into(),
        args.binseq.num_threads(),
    )?;

    write_statistics(&args.output, &statistics)?;
    write_features(&args.output, &target_mapper)?;
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
