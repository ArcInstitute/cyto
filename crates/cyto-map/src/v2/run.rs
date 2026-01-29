use std::time::Instant;

use anyhow::Result;
use binseq::ParallelReader;
use cyto_cli::{
    ArgsCrispr2, ArgsGex2,
    map2::{GEOMETRY_CRISPR_FLEX_V1, GEOMETRY_GEX_FLEX_V1},
};
use cyto_io::write_features2;

use crate::v2::{
    Component, CrisprMapper, Geometry, GexMapper, Library, MapProcessor, ProbeMapper, UmiMapper,
    WhitelistMapper, initialize_output_ibus,
    stats::{InputRuntimeStatistics, write_statistics},
    utils::{build_filepaths, delete_empty_ibus},
};

fn parse_geometry_with_default(geometry: Option<&str>, default: &str) -> Result<Geometry> {
    if let Some(g) = geometry {
        Ok(g.parse()?)
    } else {
        Ok(default.parse()?)
    }
}

pub fn run_gex2(args: &ArgsGex2) -> Result<()> {
    // Parse geometry from args
    let geometry = if let Some(preset) = args.map2.preset {
        Ok(preset.into_geometry_str().parse()?)
    } else {
        parse_geometry_with_default(args.map2.geometry.as_deref(), GEOMETRY_GEX_FLEX_V1)
    }?;

    // Load mappers (unpositioned)
    let probe = ProbeMapper::from_file(&args.map2.probes)?;
    let whitelist = WhitelistMapper::from_file(&args.map2.whitelist, args.runtime.num_threads)?;
    let gex = GexMapper::from_file(&args.gex.gex_filepath)?;

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
    write_features2(&args.output.outdir, &gex)?;

    // Build output handles
    let bijection = probe.bijection();
    let filepaths = build_filepaths(&args.output.outdir, &bijection)?;
    let writers = initialize_output_ibus(&filepaths, &resolved)?;

    // Process
    let proc = MapProcessor::new(umi, probe, whitelist, gex, writers, bijection);
    let mut runstats = Vec::default();
    for (input_id, reader) in args.input.to_binseq_readers()?.into_iter().enumerate() {
        let num_records = reader.num_records()?;

        let start = Instant::now();
        reader.process_parallel(proc.clone(), args.runtime.num_threads())?;
        let elapsed = start.elapsed();

        runstats.push(InputRuntimeStatistics {
            input_id,
            records: num_records,
            elapsed_time: elapsed.as_secs_f64(),
            mrps: num_records as f64 / elapsed.as_micros() as f64,
        });
    }
    let mapstats = proc.stats();

    // Write statistics
    write_statistics(&args.output.outdir, &libstats, mapstats, &runstats)?;

    // Delete empty IBUs
    delete_empty_ibus(&filepaths)?;

    Ok(())
}

pub fn run_crispr2(args: &ArgsCrispr2) -> Result<()> {
    // Parse geometry from args
    let geometry = if let Some(geometry) = args.map2.preset {
        Ok(geometry.into_geometry_str().parse()?)
    } else {
        parse_geometry_with_default(args.map2.geometry.as_deref(), GEOMETRY_CRISPR_FLEX_V1)
    }?;

    // Load mappers (unpositioned)
    let probe = ProbeMapper::from_file(&args.map2.probes)?;
    let whitelist = WhitelistMapper::from_file(&args.map2.whitelist, args.runtime.num_threads)?;
    let crispr = CrisprMapper::from_file(&args.crispr.guides_filepath)?;

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
    write_features2(&args.output.outdir, &crispr)?;

    // Build output handles
    let bijection = probe.bijection();
    let filepaths = build_filepaths(&args.output.outdir, &bijection)?;
    let writers = initialize_output_ibus(&filepaths, &resolved)?;

    // Process
    let proc = MapProcessor::new(umi, probe, whitelist, crispr, writers, bijection);
    let mut runstats = Vec::default();
    for (input_id, reader) in args.input.to_binseq_readers()?.into_iter().enumerate() {
        let num_records = reader.num_records()?;

        let start = Instant::now();
        reader.process_parallel(proc.clone(), args.runtime.num_threads())?;
        let elapsed = start.elapsed();

        runstats.push(InputRuntimeStatistics {
            input_id,
            records: num_records,
            elapsed_time: elapsed.as_secs_f64(),
            mrps: num_records as f64 / elapsed.as_micros() as f64,
        });
    }
    let mapstats = proc.stats();

    // Write statistics
    write_statistics(&args.output.outdir, &libstats, mapstats, &runstats)?;

    // Delete empty IBUs
    delete_empty_ibus(&filepaths)?;

    Ok(())
}
