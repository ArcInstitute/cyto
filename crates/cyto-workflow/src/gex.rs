use anyhow::Result;
use log::info;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::utils::{ibu_steps, identify_ibu_files};
use cyto_cli::workflow::GexMappingCommand;
use cyto_ibu_barcode_correct::Whitelist;

pub const DEFAULT_OUTPUT_BASENAME: &str = "output";

pub fn run(args: &GexMappingCommand) -> Result<()> {
    args.wf_args.validate_requirements(args.mode())?;

    info!("Running GEX Mapping Workflow");
    cyto_map::gex::run(&args.gex_args)?;

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

        ibu_files.par_iter().try_for_each(|path| -> Result<()> {
            ibu_steps(
                path,
                &args.gex_args.output.outdir,
                &args.wf_args,
                whitelist.clone(),
                args.mode(),
            )
        })?;
    } else {
        let ibu_file = format!(
            "{}/ibu/{}.ibu",
            args.gex_args.output.outdir, DEFAULT_OUTPUT_BASENAME
        );
        ibu_steps(
            &ibu_file,
            &args.gex_args.output.outdir,
            &args.wf_args,
            whitelist,
            args.mode(),
        )?;
    }

    Ok(())
}
