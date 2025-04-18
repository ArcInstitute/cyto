use anyhow::Result;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::utils::{ibu_steps, identify_ibu_files};
use crate::cli::workflow::FlexMappingCommand;
use crate::commands::ibu::correct::Whitelist;
use crate::commands::map as map_command;

pub fn run(args: &FlexMappingCommand) -> Result<()> {
    eprintln!(">> Running Flex Mapping Command");
    map_command::flex::run(&args.flex_args)?;

    let whitelist = if args.wf_args.skip_barcode {
        None
    } else {
        let whitelist = Whitelist::from_path(&args.wf_args.whitelist)?;
        Some(whitelist)
    };

    // Need to handle multiple output IBU files
    if args.flex_args.probe.probes_filepath.is_some() {
        // Identify all output IBU files
        let ibu_files = identify_ibu_files(&args.flex_args.output.prefix)?;

        ibu_files.par_iter().try_for_each(|path| -> Result<()> {
            ibu_steps(
                path,
                &args.flex_args.output.prefix,
                &args.wf_args,
                whitelist.clone(),
            )
        })?;
    } else {
        let ibu_file = format!("{}.ibu", args.flex_args.output.prefix);
        ibu_steps(
            &ibu_file,
            &args.flex_args.output.prefix,
            &args.wf_args,
            whitelist,
        )?;
    }

    Ok(())
}
