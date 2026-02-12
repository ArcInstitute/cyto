use std::time::Instant;

use anyhow::Result;
use binseq::ParallelReader;
use cyto_cli::{
    ArgsCrispr, ArgsGex,
    map::MultiPairedInput,
    map::{GEOMETRY_CRISPR_FLEX_V1, GEOMETRY_GEX_FLEX_V1},
};
use cyto_io::write_features;
use log::info;

use crate::{
    Component, CrisprMapper, Geometry, GexMapper, Library, MapProcessor, Mapper, ProbeMapper,
    UmiMapper, WhitelistMapper, initialize_output_ibus,
    stats::{InputRuntimeStatistics, write_statistics},
    utils::{build_filepaths, delete_sparse_ibus},
};

fn parse_geometry_with_default(geometry: Option<&str>, default: &str) -> Result<Geometry> {
    if let Some(g) = geometry {
        info!("Using geometry: `{g}`");
        Ok(g.parse()?)
    } else {
        info!("Using default geometry: `{default}`");
        Ok(default.parse()?)
    }
}

fn process_input<M>(
    inputs: &MultiPairedInput,
    mut proc: MapProcessor<M>,
    threads: usize,
) -> Result<Vec<InputRuntimeStatistics>>
where
    M: Mapper + Send + Sync + 'static,
{
    let mut runstats = Vec::default();
    if inputs.is_binseq() {
        for (input_id, reader) in inputs.to_binseq_readers()?.into_iter().enumerate() {
            let start = Instant::now();
            reader.process_parallel(proc.clone(), threads)?;
            let elapsed_sec = start.elapsed().as_secs_f64();
            runstats.push(InputRuntimeStatistics {
                input_id,
                elapsed_sec,
            });
        }
    } else {
        let collection = inputs.to_paraseq_collection()?;
        let start = Instant::now();
        collection.process_parallel_paired(&mut proc, threads, None)?;
        let elapsed_sec = start.elapsed().as_secs_f64();
        runstats.push(InputRuntimeStatistics {
            input_id: 0,
            elapsed_sec,
        });
    }
    proc.finish_pbar();
    Ok(runstats)
}

pub fn run_gex(args: &ArgsGex) -> Result<()> {
    // Parse geometry from args
    let geometry = if let Some(preset) = args.map.preset {
        let geometry_str = preset.into_geometry_str();
        info!("Using preset ({preset:?}) geometry: `{geometry_str}`");
        Ok(geometry_str.parse()?)
    } else {
        parse_geometry_with_default(args.map.geometry.as_deref(), GEOMETRY_GEX_FLEX_V1)
    }?;

    // Load mappers (unpositioned)
    let probe = if let Some(regex) = args.map.probe_regex() {
        ProbeMapper::from_file_with_alias_regex(
            args.map.probe_path(),
            args.map.exact,
            args.map.remap_window(),
            regex,
        )
    } else {
        ProbeMapper::from_file(
            args.map.probe_path(),
            args.map.exact,
            args.map.remap_window(),
        )
    }?;
    let whitelist = WhitelistMapper::from_file(
        args.map.whitelist_path(),
        args.map.exact,
        args.map.remap_window(),
        args.runtime.num_threads,
    )?;
    let gex = GexMapper::from_file(&args.gex.gex_filepath, args.map.remap_window())?;

    // Resolve geometry
    let resolved = geometry.resolve(|component| match component {
        Component::Barcode => Some(whitelist.seq_len()),
        Component::Probe => Some(probe.seq_len()),
        Component::Gex => Some(gex.seq_len()),
        _ => None,
    })?;

    // Finalize mappers with positions
    let probe = probe.resolve(&resolved)?;
    let whitelist = whitelist.resolve(&resolved)?;
    let gex = gex.resolve(&resolved)?;
    let umi = UmiMapper::resolve(&resolved)?;

    let libstats = vec![probe.statistics(), whitelist.statistics(), gex.statistics()];

    // write features
    write_features(&args.output.outdir, &gex)?;

    // Build output handles
    let bijection = probe.bijection();
    let filepaths = build_filepaths(&args.output.outdir, &bijection)?;
    let writers = initialize_output_ibus(&filepaths, &resolved)?;

    // Process
    let proc = MapProcessor::new(umi, probe, whitelist, gex, writers, bijection);
    let runstats = process_input(&args.input, proc.clone(), args.runtime.num_threads())?;
    let mapstats = proc.stats();

    // Write statistics
    write_statistics(&args.output.outdir, &libstats, mapstats, &runstats)?;

    // Delete sparse IBUs
    delete_sparse_ibus(&filepaths, args.output.min_ibu_records)?;

    Ok(())
}

pub fn run_crispr(args: &ArgsCrispr) -> Result<()> {
    // Parse geometry from args
    let geometry = if let Some(geometry) = args.map.preset {
        let geometry_str = geometry.into_geometry_str();
        info!("Using preset ({geometry:?}) geometry: `{geometry_str}`");
        Ok(geometry_str.parse()?)
    } else {
        parse_geometry_with_default(args.map.geometry.as_deref(), GEOMETRY_CRISPR_FLEX_V1)
    }?;

    // Load mappers (unpositioned)
    let probe = if let Some(regex) = args.map.probe_regex() {
        ProbeMapper::from_file_with_alias_regex(
            args.map.probe_path(),
            args.map.exact,
            args.map.remap_window(),
            regex,
        )
    } else {
        ProbeMapper::from_file(
            args.map.probe_path(),
            args.map.exact,
            args.map.remap_window(),
        )
    }?;
    let whitelist = WhitelistMapper::from_file(
        args.map.whitelist_path(),
        args.map.exact,
        args.map.remap_window(),
        args.runtime.num_threads,
    )?;
    let crispr = CrisprMapper::from_file(
        &args.crispr.guides_filepath,
        args.map.exact,
        args.map.remap_window(),
    )?;

    // Resolve geometry
    let resolved = geometry.resolve(|component| match component {
        Component::Barcode => Some(whitelist.seq_len()),
        Component::Probe => Some(probe.seq_len()),
        Component::Anchor => crispr.anchor_len(),
        Component::Protospacer => Some(crispr.protospacer_len()),
        _ => None,
    })?;

    // Finalize mappers with positions
    let probe = probe.resolve(&resolved)?;
    let whitelist = whitelist.resolve(&resolved)?;
    let crispr = crispr.resolve(&resolved)?;
    let umi = UmiMapper::resolve(&resolved)?;

    let libstats = vec![
        probe.statistics(),
        whitelist.statistics(),
        crispr.statistics(),
    ];

    // Write features
    write_features(&args.output.outdir, &crispr)?;

    // Build output handles
    let bijection = probe.bijection();
    let filepaths = build_filepaths(&args.output.outdir, &bijection)?;
    let writers = initialize_output_ibus(&filepaths, &resolved)?;

    // Process
    let proc = MapProcessor::new(umi, probe, whitelist, crispr, writers, bijection);
    let runstats = process_input(&args.input, proc.clone(), args.runtime.num_threads())?;
    let mapstats = proc.stats();

    // Write statistics
    write_statistics(&args.output.outdir, &libstats, mapstats, &runstats)?;

    // Delete sparse IBUs
    delete_sparse_ibus(&filepaths, args.output.min_ibu_records)?;

    Ok(())
}
