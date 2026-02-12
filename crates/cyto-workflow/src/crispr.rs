use std::{path::Path, sync::Arc, time::Instant};

use anyhow::{Result, bail};
use log::{info, trace};
use parking_lot::Mutex;
use rayon::{
    ThreadPoolBuilder,
    iter::{IntoParallelRefIterator, ParallelIterator},
};

use cyto_cli::workflow::CrisprMappingCommand;

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
    cyto_map::run_crispr(&args.crispr_args)?;
    let elapsed = start.elapsed();
    all_timings
        .lock()
        .push(ModuleTiming::new("All-Barcodes", Module::Mapping, elapsed));

    // Identify all output IBU files
    let ibu_files = identify_ibu_files(&args.crispr_args.output.outdir)?;
    if ibu_files.is_empty() {
        bail!(
            "No IBU files found after mapping. All probes may have been filtered by --min-ibu-records threshold."
        );
    }
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
                args.mode(),
                Some(args.geomux_args),
                threads_per_file,
            )?;
            all_timings.lock().extend_from_slice(&timings);
            Ok(())
        })
    })?;

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
