use anyhow::Result;
use glob::glob;

use crate::cli::ibu::ArgsCount;
use crate::cli::ibu::ArgsSort;
use crate::cli::workflow::FlexMappingCommand;
use crate::commands::ibu as ibu_command;
use crate::commands::map as map_command;

fn sort_and_count(ibu_path: &str, prefix: &str) -> Result<()> {
    let sort_path = ibu_path.replace(".ibu", ".sort.ibu");
    let sort_args = ArgsSort::from_wf_path(ibu_path, &sort_path, 1);

    eprintln!(">> Sorting {ibu_path} -> {sort_path}");
    ibu_command::sort::run(&sort_args)?;

    eprintln!(">> Removing unsorted file: {ibu_path}");
    std::fs::remove_file(ibu_path)?;

    let feature_path = format!("{prefix}.features.tsv");
    let count_path = sort_path.replace(".sort.ibu", ".counts.tsv");
    let count_args = ArgsCount::from_wf_path(&sort_path, &count_path, &feature_path, 1);

    eprintln!(">> Counting {sort_path} -> {count_path}");
    ibu_command::count::run(&count_args)?;

    Ok(())
}

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
