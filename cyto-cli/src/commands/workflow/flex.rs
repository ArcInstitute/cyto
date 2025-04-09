use anyhow::Result;
use glob::glob;

use super::utils::sort_and_count;
use crate::cli::workflow::FlexMappingCommand;
use crate::commands::map as map_command;

pub fn run(args: &FlexMappingCommand) -> Result<()> {
    eprintln!(">> Running Flex Mapping Command");
    map_command::flex::run(&args.flex_args)?;

    // Need to handle multiple output IBU files
    if args.flex_args.probe.probes_filepath.is_some() {
        // Identify all output IBU files
        let ibu_files = glob(&format!("{}*.ibu", args.flex_args.output.prefix))?;
        for path in ibu_files {
            let path = path?
                .into_os_string()
                .into_string()
                .expect("Could not convert path to string");

            sort_and_count(&path, &args.flex_args.output.prefix)?;
        }
    } else {
        let ibu_file = format!("{}.ibu", args.flex_args.output.prefix);
        sort_and_count(&ibu_file, &args.flex_args.output.prefix)?;
    }

    Ok(())
}
