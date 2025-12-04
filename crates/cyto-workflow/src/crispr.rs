use std::{path::Path, sync::Arc, time::Instant};

use anyhow::Result;
use log::{info, trace};
use parking_lot::Mutex;
use rayon::{
    ThreadPoolBuilder,
    iter::{IntoParallelRefIterator, ParallelIterator},
};

use cyto_cli::workflow::CrisprMappingCommand;
use cyto_ibu_barcode_correct::Whitelist;

use crate::{
    timing::{Module, ModuleTiming},
    utils::{
        RefWorkflowCommand, ibu_steps, identify_ibu_files, remove_ibu_dir, write_done_file,
        write_timings_file,
    },
};

pub fn run(args: &CrisprMappingCommand) -> Result<()> {
    args.wf_args.validate_requirements(args.mode())?;

    // initialize the timing vector
    let all_timings = Arc::new(Mutex::new(Vec::new()));

    info!("Running CRISPR Mapping Workflow");
    let start = Instant::now();
    cyto_map::crispr::run(&args.crispr_args)?;
    let elapsed = start.elapsed();
    all_timings
        .lock()
        .push(ModuleTiming::new("All-Barcodes", Module::Mapping, elapsed));

    let whitelist = if args.wf_args.skip_barcode {
        None
    } else {
        let whitelist = Whitelist::from_path(&args.wf_args.whitelist)?;
        Some(whitelist)
    };

    // Need to handle multiple output IBU files
    if args.crispr_args.probe.probes_filepath.is_some() {
        // Identify all output IBU files
        let ibu_files = identify_ibu_files(&args.crispr_args.output.outdir)?;
        let total_threads = args.crispr_args.runtime.num_threads();
        let threads_per_file = (total_threads / ibu_files.len()).max(1);

        let pool = ThreadPoolBuilder::new()
            .num_threads(total_threads)
            .build()?;

        trace!(
            "Total number of subprocesses ({}) over ({}) threads",
            ibu_files.len(),
            total_threads
        );
        pool.install(|| {
            ibu_files.par_iter().try_for_each(|path| -> Result<()> {
                let timings = ibu_steps(
                    path,
                    &args.crispr_args.output.outdir,
                    &args.wf_args,
                    whitelist.clone(),
                    args.mode(),
                    Some(args.geomux_args),
                    threads_per_file,
                )?;
                all_timings.lock().extend_from_slice(&timings);
                Ok(())
            })
        })?;
    } else {
        let ibu_file = format!("{}/ibu/output.ibu", args.crispr_args.output.outdir);
        let timings = ibu_steps(
            &ibu_file,
            &args.crispr_args.output.outdir,
            &args.wf_args,
            whitelist,
            args.mode(),
            Some(args.geomux_args),
            args.crispr_args.runtime.num_threads(),
        )?;
        all_timings.lock().extend_from_slice(&timings);
    }

    if !args.wf_args.keep_ibu {
        remove_ibu_dir(Path::new(&args.crispr_args.output.outdir).join("ibu"))?;
    }

    write_done_file(
        &args.crispr_args.output.outdir,
        &RefWorkflowCommand::CrisprMapping(args),
    )?;
    write_timings_file(&args.crispr_args.output.outdir, &all_timings.lock())?;

    Ok(())
}
