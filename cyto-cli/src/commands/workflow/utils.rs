use anyhow::Result;

use crate::cli::ibu::ArgsCount;
use crate::cli::ibu::ArgsSort;
use crate::commands::ibu as ibu_command;

pub fn sort_and_count(ibu_path: &str, prefix: &str) -> Result<()> {
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
