use std::{path::Path, sync::Arc, time::Instant};

use anyhow::Result;
use log::{info, trace};
use parking_lot::Mutex;
use rayon::{
    ThreadPoolBuilder,
    iter::{IntoParallelRefIterator, ParallelIterator},
};

use cyto_cli::workflow::GexMappingCommand;
use cyto_ibu_barcode_correct::Whitelist;

use crate::{
    timing::{Module, ModuleTiming},
    utils::{
        RefWorkflowCommand, ibu_steps, identify_ibu_files, remove_ibu_dir, write_done_file,
        write_timings_file,
    },
};

pub const DEFAULT_OUTPUT_BASENAME: &str = "output";

pub fn run(args: &GexMappingCommand) -> Result<()> {
    args.wf_args.validate_requirements(args.mode())?;

    // initialize the timing vector
    let all_timings = Arc::new(Mutex::new(Vec::new()));

    info!("Running GEX Mapping Workflow");
    let start = Instant::now();
    cyto_map::gex::run(&args.gex_args)?;
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
    if args.gex_args.probe.probes_filepath.is_some() {
        // Identify all output IBU files
        let ibu_files = identify_ibu_files(&args.gex_args.output.outdir)?;
        let total_threads = args.gex_args.runtime.num_threads();
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
                    &args.gex_args.output.outdir,
                    &args.wf_args,
                    whitelist.clone(),
                    args.mode(),
                    None,
                    threads_per_file,
                )?;
                all_timings.lock().extend_from_slice(&timings);
                Ok(())
            })
        })?;
    } else {
        let ibu_file = format!(
            "{}/ibu/{}.ibu",
            args.gex_args.output.outdir, DEFAULT_OUTPUT_BASENAME
        );
        let timings = ibu_steps(
            &ibu_file,
            &args.gex_args.output.outdir,
            &args.wf_args,
            whitelist,
            args.mode(),
            None,
            args.gex_args.runtime.num_threads(),
        )?;
        all_timings.lock().extend_from_slice(&timings);
    }

    if !args.wf_args.keep_ibu {
        remove_ibu_dir(Path::new(&args.gex_args.output.outdir).join("ibu"))?;
    }

    write_done_file(
        &args.gex_args.output.outdir,
        &RefWorkflowCommand::GexMapping(args),
    )?;
    write_timings_file(&args.gex_args.output.outdir, &all_timings.lock())?;

    Ok(())
}
