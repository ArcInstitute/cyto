use anyhow::Result;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::utils::{identify_ibu_files, sort_and_count};
use crate::cli::workflow::FlexMappingCommand;
use crate::commands::map as map_command;

pub fn run(args: &FlexMappingCommand) -> Result<()> {
    eprintln!(">> Running Flex Mapping Command");
    map_command::flex::run(&args.flex_args)?;

    // Need to handle multiple output IBU files
    if args.flex_args.probe.probes_filepath.is_some() {
        // Identify all output IBU files
        let ibu_files = identify_ibu_files(&args.flex_args.output.prefix)?;

        ibu_files.par_iter().try_for_each(|path| -> Result<()> {
            sort_and_count(path, &args.flex_args.output.prefix)
        })?;
    } else {
        let ibu_file = format!("{}.ibu", args.flex_args.output.prefix);
        sort_and_count(&ibu_file, &args.flex_args.output.prefix)?;
    }

    Ok(())
}
