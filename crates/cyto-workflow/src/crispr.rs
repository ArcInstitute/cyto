use std::path::Path;

use anyhow::Result;
use log::{info, trace};
use rayon::{
    ThreadPoolBuilder,
    iter::{IntoParallelRefIterator, ParallelIterator},
};

use cyto_cli::workflow::CrisprMappingCommand;
use cyto_ibu_barcode_correct::Whitelist;

use crate::utils::{
    RefWorkflowCommand, ibu_steps, identify_ibu_files, remove_ibu_dir, write_done_file,
};

pub fn run(args: &CrisprMappingCommand) -> Result<()> {
    args.wf_args.validate_requirements(args.mode())?;

    info!("Running CRISPR Mapping Workflow");
    cyto_map::crispr::run(&args.crispr_args)?;

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
                ibu_steps(
                    path,
                    &args.crispr_args.output.outdir,
                    &args.wf_args,
                    whitelist.clone(),
                    args.mode(),
                    Some(args.geomux_args),
                    threads_per_file,
                )
            })
        })?;
    } else {
        let ibu_file = format!("{}/ibu/output.ibu", args.crispr_args.output.outdir);
        ibu_steps(
            &ibu_file,
            &args.crispr_args.output.outdir,
            &args.wf_args,
            whitelist,
            args.mode(),
            Some(args.geomux_args),
            args.crispr_args.runtime.num_threads(),
        )?;
    }

    if !args.wf_args.keep_ibu {
        remove_ibu_dir(Path::new(&args.crispr_args.output.outdir).join("ibu"))?;
    }

    write_done_file(
        &args.crispr_args.output.outdir,
        &RefWorkflowCommand::CrisprMapping(args),
    )?;

    Ok(())
}
