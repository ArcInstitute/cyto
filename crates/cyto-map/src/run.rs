use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Result, bail};
use binseq::ParallelReader;
use cyto_cli::{
    ArgsCrispr, ArgsGex, ArgsOutput,
    map::MultiPairedInput,
    map::{GEOMETRY_CRISPR_FLEX_V1, GEOMETRY_GEX_FLEX_V1},
};
use cyto_io::{FeatureWriter, write_features};
use log::{info, warn};

use crate::{
    Component, CrisprMapper, Geometry, GexMapper, Library, MapProcessor, Mapper, ProbeMapper,
    ResolvedGeometry, UmiMapper, WhitelistMapper, initialize_output_ibus,
    stats::{InputRuntimeStatistics, LibraryStatistics, write_statistics},
    utils::{build_filepath, build_filepaths, delete_sparse_ibus, initialize_output_ibu},
};

fn parse_geometry(args: &cyto_cli::map::MapOptions, default: &str) -> Result<Geometry> {
    if let Some(preset) = args.preset {
        let geometry_str = preset.into_geometry_str();
        info!("Using preset ({preset:?}) geometry: `{geometry_str}`");
        Ok(geometry_str.parse()?)
    } else if let Some(ref g) = args.geometry {
        info!("Using geometry: `{g}`");
        Ok(g.parse()?)
    } else {
        info!("Using default geometry: `{default}`");
        Ok(default.parse()?)
    }
}

fn load_probe(
    args: &cyto_cli::map::MapOptions,
) -> Result<Option<ProbeMapper<crate::Unpositioned>>> {
    let Some(probe_path) = args.probe_path() else {
        return Ok(None);
    };
    let probe = if let Some(regex) = args.probe_regex() {
        ProbeMapper::from_file_with_alias_regex(probe_path, args.exact, args.remap_window(), regex)
    } else {
        ProbeMapper::from_file(probe_path, args.exact, args.remap_window())
    }?;
    Ok(Some(probe))
}

/// Validate that the geometry and probe file are consistent.
///
/// - If the geometry contains `[probe]` but no probe file is provided, this is an error
///   because the probe region's length is unknown and downstream offsets will be wrong.
/// - If a probe file is provided but the geometry has no `[probe]`, we warn that the
///   probe file will be unused and demultiplexing will be skipped.
fn validate_probe_geometry(geometry: &Geometry, has_probe_file: bool) -> Result<()> {
    let geometry_has_probe = geometry.has_component(Component::Probe);
    if geometry_has_probe && !has_probe_file {
        bail!(
            "geometry contains [probe] but no probe file was provided. \
             Either provide a probe file with --probes or use a geometry without [probe]."
        );
    }
    if !geometry_has_probe && has_probe_file {
        warn!(
            "probe file provided but geometry does not contain [probe]; \
             probes will not be used for demultiplexing."
        );
    }
    Ok(())
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
    let geometry = parse_geometry(&args.map, GEOMETRY_GEX_FLEX_V1)?;
    let probe = load_probe(&args.map)?;
    validate_probe_geometry(&geometry, probe.is_some())?;

    // Load mappers (unpositioned)
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
        Component::Probe => probe.as_ref().map(ProbeMapper::seq_len),
        Component::Gex => Some(gex.seq_len()),
        _ => None,
    })?;

    // Finalize mappers with positions
    let probe = probe.map(|p| p.resolve(&resolved)).transpose()?;
    let whitelist = whitelist.resolve(&resolved)?;
    let gex = gex.resolve(&resolved)?;
    let umi = UmiMapper::resolve(&resolved)?;

    run_pipeline(
        probe,
        whitelist,
        gex,
        umi,
        &resolved,
        &args.input,
        &args.output,
        args.runtime.num_threads(),
    )
}

pub fn run_crispr(args: &ArgsCrispr) -> Result<()> {
    let geometry = parse_geometry(&args.map, GEOMETRY_CRISPR_FLEX_V1)?;
    let probe = load_probe(&args.map)?;
    validate_probe_geometry(&geometry, probe.is_some())?;

    // Load mappers (unpositioned)
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
        Component::Probe => probe.as_ref().map(ProbeMapper::seq_len),
        Component::Anchor => crispr.anchor_len(),
        Component::Protospacer => Some(crispr.protospacer_len()),
        _ => None,
    })?;

    // Finalize mappers with positions
    let probe = probe.map(|p| p.resolve(&resolved)).transpose()?;
    let whitelist = whitelist.resolve(&resolved)?;
    let crispr = crispr.resolve(&resolved)?;
    let umi = UmiMapper::resolve(&resolved)?;

    run_pipeline(
        probe,
        whitelist,
        crispr,
        umi,
        &resolved,
        &args.input,
        &args.output,
        args.runtime.num_threads(),
    )
}

/// Shared pipeline: build outputs, process reads, write stats, clean up sparse IBUs.
#[allow(clippy::too_many_arguments)]
fn run_pipeline<M>(
    probe: Option<ProbeMapper>,
    whitelist: WhitelistMapper,
    feature: M,
    umi: UmiMapper,
    resolved: &ResolvedGeometry,
    input: &MultiPairedInput,
    output: &ArgsOutput,
    num_threads: usize,
) -> Result<()>
where
    M: Mapper + Library + Send + Sync + 'static,
    for<'a> M: FeatureWriter<'a>,
{
    // Build library statistics
    let libstats: Vec<LibraryStatistics> = {
        let mut libstats = Vec::new();
        if let Some(ref probe) = probe {
            libstats.push(probe.statistics());
        }
        libstats.push(whitelist.statistics());
        libstats.push(feature.statistics());
        libstats
    };

    // Write features
    write_features(&output.outdir, &feature)?;

    // Build output handles and processor
    let (proc, filepaths): (MapProcessor<M>, Vec<PathBuf>) = if let Some(probe) = probe {
        let bijection = probe.bijection();
        let filepaths = build_filepaths(&output.outdir, &bijection)?;
        let writers = initialize_output_ibus(&filepaths, resolved)?;
        (
            MapProcessor::probed(umi, probe, whitelist, feature, writers, bijection),
            filepaths,
        )
    } else {
        let filepath = build_filepath(&output.outdir, None);
        let writer = initialize_output_ibu(&filepath, resolved)?;
        (
            MapProcessor::unprobed(umi, whitelist, feature, writer),
            vec![filepath],
        )
    };

    // Process
    let runstats = process_input(input, proc.clone(), num_threads)?;
    let mapstats = proc.stats();

    // Write statistics
    write_statistics(&output.outdir, &libstats, mapstats, &runstats)?;

    // Delete sparse IBUs
    delete_sparse_ibus(&filepaths, output.min_ibu_records)?;

    Ok(())
}
