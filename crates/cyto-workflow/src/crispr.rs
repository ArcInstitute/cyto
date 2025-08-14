use anyhow::Result;
use log::info;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::utils::{ibu_steps, identify_ibu_files};
use cyto_cli::workflow::CrisprMappingCommand;
use cyto_ibu_barcode_correct::Whitelist;

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

        ibu_files.par_iter().try_for_each(|path| -> Result<()> {
            ibu_steps(
                path,
                &args.crispr_args.output.outdir,
                &args.wf_args,
                whitelist.clone(),
                args.mode(),
            )
        })?;
    } else {
        let ibu_file = format!("{}/ibu/output.ibu", args.crispr_args.output.outdir);
        ibu_steps(
            &ibu_file,
            &args.crispr_args.output.outdir,
            &args.wf_args,
            whitelist,
            args.mode(),
        )?;
    }

    Ok(())
}
