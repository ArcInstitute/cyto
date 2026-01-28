use std::time::Instant;

use anyhow::{Result, anyhow};
use binseq::ParallelReader;
use cyto_cli::ArgsGex;

use crate::v2::{
    Component, GEOMETRY_GEX_FLEX_V1, Geometry, GexMapper, MapProcessor, ProbeMapper, UmiMapper,
    WhitelistMapper, initialize_output_ibus,
};

pub fn run_gex(args: &ArgsGex) -> Result<()> {
    // Parse geometry
    let geometry = GEOMETRY_GEX_FLEX_V1.parse::<Geometry>()?;

    // 2. Load mappers (unpositioned)
    let probe = ProbeMapper::from_file(
        args.probe
            .probes_filepath
            .as_ref()
            .expect("Missing probes filepath"),
    )?;
    let whitelist = WhitelistMapper::from_file(
        &args
            .map
            .whitelist
            .as_ref()
            .expect("Missing whitelist filepath"),
        args.runtime.num_threads,
    )?;
    let gex = GexMapper::from_file(&args.gex.gex_filepath)?;

    // 3. Resolve geometry
    let resolved = geometry.resolve(|component| match component {
        Component::Barcode => Some(whitelist.seq_len()),
        Component::Probe => Some(probe.seq_len()),
        Component::Gex => Some(gex.seq_len()),
        Component::Umi => None,         // explicit in geometry
        Component::Anchor => None,      // not used in GEX
        Component::Protospacer => None, // not used in GEX
    })?;

    // 4. Finalize mappers with positions
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

    // 5. build output handles
    let bijection = probe.bijection();
    let writers = initialize_output_ibus(&args.output.outdir, &resolved, &bijection)?;

    // 6. Process
    let proc = MapProcessor::new(umi, probe, whitelist, gex, writers, bijection);
    for reader in args.input.to_binseq_readers()? {
        let start = Instant::now();
        reader.process_parallel(proc.clone(), args.runtime.num_threads())?;
        let elapsed = start.elapsed();
        eprintln!(
            "Throughput: {:.3}M/s",
            proc.total() as f64 / elapsed.as_micros() as f64
        );
    }

    proc.pprint();

    Ok(())
}
