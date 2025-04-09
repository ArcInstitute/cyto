use anyhow::Result;
use glob::glob;

use super::utils::sort_and_count;
use crate::cli::workflow::CrisprMappingCommand;
use crate::commands::map as map_command;

pub fn run(args: &CrisprMappingCommand) -> Result<()> {
    eprintln!(">> Running CRISPR Mapping Command");
    map_command::crispr::run(&args.crispr_args)?;

    // Need to handle multiple output IBU files
    if args.crispr_args.probe.probes_filepath.is_some() {
        // Identify all output IBU files
        let ibu_files = glob(&format!("{}*.ibu", args.crispr_args.output.prefix))?;
        for path in ibu_files {
            let path = path?
                .into_os_string()
                .into_string()
                .expect("Could not convert path to string");

            sort_and_count(&path, &args.crispr_args.output.prefix)?;
        }
    } else {
        let ibu_file = format!("{}.ibu", args.crispr_args.output.prefix);
        sort_and_count(&ibu_file, &args.crispr_args.output.prefix)?;
    }

    Ok(())
}
