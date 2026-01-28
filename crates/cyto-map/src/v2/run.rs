use std::time::Instant;

use anyhow::{Result, anyhow};
use binseq::ParallelReader;
use cyto_cli::{ArgsCrispr2, ArgsGex2};

use crate::v2::{
    Component, CrisprMapper, GEOMETRY_CRISPR_PROPERSEQ, GEOMETRY_GEX_FLEX_V1, Geometry, GexMapper,
    Library, MapProcessor, ProbeMapper, UmiMapper, WhitelistMapper, initialize_output_ibus,
    stats::{InputRuntimeStatistics, write_statistics},
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
    let geometry =
        parse_geometry_with_default(args.map2.geometry.as_deref(), GEOMETRY_GEX_FLEX_V1)?;

    // Load mappers (unpositioned)
    let probe = ProbeMapper::from_file(
        args.map2
            .probes
            .as_ref()
            .expect("Missing probes filepath - not yet implemented without probes"),
    )?;
    let whitelist = WhitelistMapper::from_file(
        args.map2
            .whitelist
            .as_ref()
            .expect("Missing whitelist filepath - not yet implemented without whitelist"),
        args.runtime.num_threads,
    )?;
    let gex = GexMapper::from_file(&args.gex.gex_filepath)?;

    // Resolve geometry
    let resolved = geometry.resolve(|component| match component {
        Component::Barcode => Some(whitelist.seq_len()),
        Component::Probe => Some(probe.seq_len()),
        Component::Gex => Some(gex.seq_len()),
        Component::Umi => None,
        Component::Anchor => None,
        Component::Protospacer => None,
    })?;

    // Finalize mappers with positions
    let probe_region = resolved
        .get(Component::Probe)
        .ok_or_else(|| anyhow!("geometry missing [probe]"))?;
    let probe = probe.with_position(probe_region.offset, probe_region.mate);

    let barcode_region = resolved
        .get(Component::Barcode)
        .ok_or_else(|| anyhow!("geometry missing [barcode]"))?;
    let whitelist = whitelist.with_position(barcode_region.offset, barcode_region.mate);

    let gex_region = resolved
        .get(Component::Gex)
        .ok_or_else(|| anyhow!("geometry missing [gex]"))?;
    let gex = gex.with_position(gex_region.offset, gex_region.mate);

    let umi_region = resolved
        .get(Component::Umi)
        .ok_or_else(|| anyhow!("geometry missing [umi]"))?;
    let umi = UmiMapper::new(
        umi_region.offset,
        umi_region.length.expect("length missing [umi]"),
        umi_region.mate,
    );

    let libstats = vec![probe.statistics(), whitelist.statistics(), gex.statistics()];

    // Build output handles
    let bijection = probe.bijection();
    let writers = initialize_output_ibus(&args.output.outdir, &resolved, &bijection)?;

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

    Ok(())
}

pub fn run_crispr2(args: &ArgsCrispr2) -> Result<()> {
    // Parse geometry from args
    let geometry =
        parse_geometry_with_default(args.map2.geometry.as_deref(), GEOMETRY_CRISPR_PROPERSEQ)?;

    // Load mappers (unpositioned)
    let probe = ProbeMapper::from_file(
        args.map2
            .probes
            .as_ref()
            .expect("Missing probes filepath - not yet implemented without probes"),
    )?;
    let whitelist = WhitelistMapper::from_file(
        args.map2
            .whitelist
            .as_ref()
            .expect("Missing whitelist filepath - not yet implemented without whitelist"),
        args.runtime.num_threads,
    )?;
    let crispr = CrisprMapper::from_file(&args.crispr.guides_filepath)?;

    // Resolve geometry
    let resolved = geometry.resolve(|component| match component {
        Component::Barcode => Some(whitelist.seq_len()),
        Component::Probe => Some(probe.seq_len()),
        Component::Gex => None,
        Component::Umi => None,
        Component::Anchor => crispr.anchor_len(),
        Component::Protospacer => Some(crispr.protospacer_len()),
    })?;

    // Finalize mappers with positions
    let probe_region = resolved
        .get(Component::Probe)
        .ok_or_else(|| anyhow!("geometry missing [probe]"))?;
    let probe = probe.with_position(probe_region.offset, probe_region.mate);

    let barcode_region = resolved
        .get(Component::Barcode)
        .ok_or_else(|| anyhow!("geometry missing [barcode]"))?;
    let whitelist = whitelist.with_position(barcode_region.offset, barcode_region.mate);

    let anchor_region = resolved
        .get(Component::Anchor)
        .ok_or_else(|| anyhow!("geometry missing [anchor]"))?;
    let crispr = crispr.with_position(anchor_region.offset, anchor_region.mate);

    let umi_region = resolved
        .get(Component::Umi)
        .ok_or_else(|| anyhow!("geometry missing [umi]"))?;
    let umi = UmiMapper::new(
        umi_region.offset,
        umi_region.length.expect("length missing [umi]"),
        umi_region.mate,
    );

    let libstats = vec![
        probe.statistics(),
        whitelist.statistics(),
        crispr.statistics(),
    ];

    // Build output handles
    let bijection = probe.bijection();
    let writers = initialize_output_ibus(&args.output.outdir, &resolved, &bijection)?;

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

    Ok(())
}
