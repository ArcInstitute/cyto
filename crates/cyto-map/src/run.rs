use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Result, bail};
use binseq::ParallelReader;
use cyto_cli::{ArgsCrispr, ArgsDetectCrispr, ArgsDetectGex, ArgsGex, ArgsOutput, map::MultiPairedInput};
use cyto_io::{FeatureWriter, write_features};
use log::{info, warn};

use crate::{
    Component, CrisprMapper, Geometry, GexMapper, Library, MapProcessor, Mapper, ProbeMapper,
    ResolvedGeometry, UmiMapper, WhitelistMapper, initialize_output_ibus,
    detect::{ComponentEvidence, DetectionConfig, DetectionResult, detect_crispr_geometry, detect_gex_geometry},
    stats::{InputRuntimeStatistics, LibraryStatistics, write_statistics},
    utils::{build_filepath, build_filepaths, delete_sparse_ibus, initialize_output_ibu},
    Unpositioned,
};

/// Auto-detect GEX geometry by sampling reads.
///
/// The mappers are consumed by the detection process.
/// Returns the detected geometry and remap window.
fn autodetect_gex_geometry(
    args: &cyto_cli::map::MapOptions,
    whitelist: WhitelistMapper<Unpositioned>,
    gex: GexMapper<Unpositioned>,
    probe: Option<ProbeMapper<Unpositioned>>,
    input: &MultiPairedInput,
) -> Result<(Geometry, usize)> {
    if args.geometry_auto_num_reads == 0 {
        bail!(
            "No geometry, preset, or auto-detection configured. \
             Provide --geometry, --preset, or set --geometry-auto-num-reads > 0."
        );
    }
    info!(
        "No geometry specified. Auto-detecting from {} reads...",
        args.geometry_auto_num_reads
    );
    let config = DetectionConfig {
        num_reads: args.geometry_auto_num_reads,
        min_proportion: args.geometry_auto_min_proportion,
        remap_min_proportion: args.geometry_auto_remap_min_proportion,
    };
    let result = detect_gex_geometry(whitelist, gex, probe, input, &config)?;
    log_detection_result(&result);
    Ok((result.geometry, result.remap_window))
}

/// Auto-detect CRISPR geometry by sampling reads.
///
/// The mappers are consumed by the detection process.
fn autodetect_crispr_geometry(
    args: &cyto_cli::map::MapOptions,
    whitelist: WhitelistMapper<Unpositioned>,
    crispr: CrisprMapper<Unpositioned>,
    probe: Option<ProbeMapper<Unpositioned>>,
    input: &MultiPairedInput,
) -> Result<(Geometry, usize)> {
    if args.geometry_auto_num_reads == 0 {
        bail!(
            "No geometry, preset, or auto-detection configured. \
             Provide --geometry, --preset, or set --geometry-auto-num-reads > 0."
        );
    }
    info!(
        "No geometry specified. Auto-detecting from {} reads...",
        args.geometry_auto_num_reads
    );
    let config = DetectionConfig {
        num_reads: args.geometry_auto_num_reads,
        min_proportion: args.geometry_auto_min_proportion,
        remap_min_proportion: args.geometry_auto_remap_min_proportion,
    };
    let result = detect_crispr_geometry(whitelist, crispr, probe, input, &config)?;
    log_detection_result(&result);
    Ok((result.geometry, result.remap_window))
}

/// Log detection results at info level.
fn log_detection_result(result: &DetectionResult) {
    info!(
        "Detected geometry: `{}`  (remap_window={})",
        result.geometry_string, result.remap_window
    );
    info!(
        "Detection sampled {} reads total",
        result.total_reads_sampled
    );
    for ev in &result.evidence {
        info!(
            "  [{}] {:?} pos={} count={} proportion={:.4}",
            ev.component, ev.mate, ev.position, ev.match_count, ev.match_proportion
        );
        log_top_alternatives(ev);
    }
}

/// Log top alternative positions for a component (up to 3).
fn log_top_alternatives(ev: &ComponentEvidence) {
    for &(mate, pos, count) in ev.top_positions.iter().skip(1).take(3) {
        info!("    alt: {mate:?} pos={pos} count={count}");
    }
}

pub fn run_detect_gex(args: &ArgsDetectGex) -> Result<()> {
    let whitelist = WhitelistMapper::from_file(
        &args.whitelist.whitelist,
        false,
        1,
        std::thread::available_parallelism().map_or(1, std::num::NonZero::get),
    )?;
    let gex = GexMapper::from_file(&args.gex.gex_filepath, 1)?;
    let probe = load_detect_probe(&args.probe)?;

    let config = DetectionConfig {
        num_reads: args.detection.num_reads,
        min_proportion: args.detection.min_proportion,
        remap_min_proportion: args.detection.remap_min_proportion,
    };
    let result = detect_gex_geometry(whitelist, gex, probe, &args.input, &config)?;
    log_detection_result(&result);
    print!("{}", crate::detect::format_detection_result(&result));
    Ok(())
}

pub fn run_detect_crispr(args: &ArgsDetectCrispr) -> Result<()> {
    let whitelist = WhitelistMapper::from_file(
        &args.whitelist.whitelist,
        false,
        1,
        std::thread::available_parallelism().map_or(1, std::num::NonZero::get),
    )?;
    let crispr = CrisprMapper::from_file(
        &args.crispr.guides_filepath,
        false,
        1,
    )?;
    let probe = load_detect_probe(&args.probe)?;

    let config = DetectionConfig {
        num_reads: args.detection.num_reads,
        min_proportion: args.detection.min_proportion,
        remap_min_proportion: args.detection.remap_min_proportion,
    };
    let result = detect_crispr_geometry(whitelist, crispr, probe, &args.input, &config)?;
    log_detection_result(&result);
    print!("{}", crate::detect::format_detection_result(&result));
    Ok(())
}

/// Load a probe mapper for detect commands (exact=false, window=1).
fn load_detect_probe(
    probe_opts: &cyto_cli::map::ProbeOptions,
) -> Result<Option<ProbeMapper<Unpositioned>>> {
    let Some(ref probe_path) = probe_opts.probes else {
        return Ok(None);
    };
    let probe = if let Some(ref regex) = probe_opts.probe_regex {
        ProbeMapper::from_file_with_alias_regex(probe_path, false, 1, regex)
    } else {
        ProbeMapper::from_file(probe_path, false, 1)
    }?;
    Ok(Some(probe))
}

fn load_probe_with_window(
    args: &cyto_cli::map::MapOptions,
    window: usize,
) -> Result<Option<ProbeMapper<Unpositioned>>> {
    let Some(probe_path) = args.probe_path() else {
        return Ok(None);
    };
    let probe = if let Some(regex) = args.probe_regex() {
        ProbeMapper::from_file_with_alias_regex(probe_path, args.exact, window, regex)
    } else {
        ProbeMapper::from_file(probe_path, args.exact, window)
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
    let has_manual_geometry = args.map.preset.is_some() || args.map.geometry.is_some();

    // When auto-detecting, mappers are loaded for detection (consumed), then
    // reloaded below for mapping. Manual geometry skips detection entirely.
    let (geometry, remap_window) = if has_manual_geometry {
        if let Some(preset) = args.map.preset {
            let geometry_str = preset.into_geometry_str();
            info!("Using preset ({preset:?}) geometry: `{geometry_str}`");
            (geometry_str.parse()?, args.map.remap_window())
        } else {
            let g = args.map.geometry.as_ref().unwrap();
            info!("Using custom geometry: `{g}`");
            (g.parse()?, args.map.remap_window())
        }
    } else {
        // Auto-detect: load mappers for detection (consumed by detect)
        let det_probe = load_probe_with_window(&args.map, 1)?;
        let det_whitelist = WhitelistMapper::from_file(
            args.map.whitelist_path(),
            args.map.exact,
            1,
            args.runtime.num_threads,
        )?;
        let det_gex = GexMapper::from_file(&args.gex.gex_filepath, 1)?;
        autodetect_gex_geometry(&args.map, det_whitelist, det_gex, det_probe, &args.input)?
    };

    let probe = load_probe_with_window(&args.map, remap_window)?;
    validate_probe_geometry(&geometry, probe.is_some())?;

    // Load mappers for mapping with detected/specified remap window
    let whitelist = WhitelistMapper::from_file(
        args.map.whitelist_path(),
        args.map.exact,
        remap_window,
        args.runtime.num_threads,
    )?;
    let gex = GexMapper::from_file(&args.gex.gex_filepath, remap_window)?;

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
    let has_manual_geometry = args.map.preset.is_some() || args.map.geometry.is_some();

    let (geometry, remap_window) = if has_manual_geometry {
        if let Some(preset) = args.map.preset {
            let geometry_str = preset.into_geometry_str();
            info!("Using preset ({preset:?}) geometry: `{geometry_str}`");
            (geometry_str.parse()?, args.map.remap_window())
        } else {
            let g = args.map.geometry.as_ref().unwrap();
            info!("Using custom geometry: `{g}`");
            (g.parse()?, args.map.remap_window())
        }
    } else {
        // Auto-detect: load mappers for detection (consumed by detect)
        let det_probe = load_probe_with_window(&args.map, 1)?;
        let det_whitelist = WhitelistMapper::from_file(
            args.map.whitelist_path(),
            args.map.exact,
            1,
            args.runtime.num_threads,
        )?;
        let det_crispr = CrisprMapper::from_file(
            &args.crispr.guides_filepath,
            args.map.exact,
            1,
        )?;
        autodetect_crispr_geometry(&args.map, det_whitelist, det_crispr, det_probe, &args.input)?
    };

    let probe = load_probe_with_window(&args.map, remap_window)?;
    validate_probe_geometry(&geometry, probe.is_some())?;

    // Load mappers for mapping with detected/specified remap window
    let whitelist = WhitelistMapper::from_file(
        args.map.whitelist_path(),
        args.map.exact,
        remap_window,
        args.runtime.num_threads,
    )?;
    let crispr = CrisprMapper::from_file(
        &args.crispr.guides_filepath,
        args.map.exact,
        remap_window,
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
