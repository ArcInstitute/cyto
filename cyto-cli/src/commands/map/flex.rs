use crate::{cli::ArgsFlex, io::write_statistics};
use anyhow::Result;
use cyto::{
    libraries::{FlexLibrary, ProbeLibrary},
    mappers::MapperOffset,
};

use super::{
    ibu_map_pairs, ibu_map_probed_pairs,
    utils::{
        build_filepath, build_filepaths, delete_empty_path, delete_empty_paths, open_handle,
        open_handles,
    },
};

fn probed_bus(args: ArgsFlex) -> Result<()> {
    let readers = args.input.into_readers()?;
    let target_mapper = FlexLibrary::from_tsv(args.flex.flex_filepath.into())?.into_mapper()?;
    let probe_mapper =
        ProbeLibrary::from_tsv(args.probe.probes_filepath.unwrap().into())?.into_mapper()?;

    // The expected start position of the probe sequence in the bus sequence
    let probe_offset = MapperOffset::RightOf(target_mapper.get_sequence_size() + args.flex.spacer);

    // Define the file path for each probe
    let filepaths = build_filepaths(&args.output.prefix, &probe_mapper)?;

    // Open a file handle for each handle
    let mut probe_writers = open_handles(&filepaths)?;

    let statistics = ibu_map_probed_pairs(
        readers,
        &mut probe_writers,
        &target_mapper,
        &probe_mapper,
        None,
        Some(probe_offset),
        args.geometry.into(),
    )?;

    // Delete the probe files if there are no mapped reads
    delete_empty_paths(&filepaths)?;

    write_statistics(&args.output, &statistics)?;
    Ok(())
}

fn bus(args: ArgsFlex) -> Result<()> {
    let readers = args.input.into_readers()?;
    let target_mapper = FlexLibrary::from_tsv(args.flex.flex_filepath.into())?.into_mapper()?;

    // Define the file path for the output file
    let output_filepath = build_filepath(&args.output.prefix, None);

    // Open a file handle for the output file
    let mut handle = open_handle(&output_filepath)?;

    let statistics = ibu_map_pairs(
        readers,
        &mut handle,
        &target_mapper,
        None,
        args.geometry.into(),
    )?;

    // Delete the output file if there are no mapped reads
    delete_empty_path(&output_filepath)?;

    write_statistics(&args.output, &statistics)?;
    Ok(())
}

pub fn run(args: ArgsFlex) -> Result<()> {
    if args.probe.probes_filepath.is_some() {
        probed_bus(args)
    } else {
        bus(args)
    }
}
