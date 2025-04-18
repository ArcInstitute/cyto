use anyhow::bail;
use anyhow::Result;
use glob::glob;

use crate::cli::ibu::ArgsCorrect;
use crate::cli::ibu::ArgsCount;
use crate::cli::ibu::ArgsSort;
use crate::cli::ibu::ArgsUmi;
use crate::cli::workflow::ArgsWorkflow;
use crate::commands::ibu as ibu_command;
use crate::commands::ibu::correct::Whitelist;

pub fn identify_ibu_files(prefix: &str) -> Result<Vec<String>> {
    let ibu_files = glob(&format!("{prefix}*.ibu"))?
        .map(|path| {
            path.expect("Path is not valid")
                .into_os_string()
                .into_string()
                .expect("Could not convert path to string")
        })
        .filter(|x| !x.ends_with(".sort.ibu"))
        .collect::<Vec<_>>();
    Ok(ibu_files)
}

pub fn ibu_steps(
    ibu_path: &str,
    prefix: &str,
    wf_args: &ArgsWorkflow,
    whitelist: Option<Whitelist>,
) -> Result<()> {
    let mut sort_path = ibu_path.replace(".ibu", ".sort.ibu");
    let sort_args = ArgsSort::from_wf_path(ibu_path, &sort_path, 1);

    eprintln!(">> Sorting {ibu_path} -> {sort_path}");
    ibu_command::sort::run(&sort_args)?;

    eprintln!(">> Removing unsorted file: {ibu_path}");
    std::fs::remove_file(ibu_path)?;

    if !wf_args.skip_barcode {
        let bc_path = sort_path.replace(".sort.ibu", ".barcode.ibu");
        let barcode_args = ArgsCorrect::from_wf_path(&sort_path, &bc_path, &wf_args.whitelist);
        let Some(whitelist) = whitelist else {
            bail!("Whitelist is required for barcode correction");
        };

        eprintln!(">> Barcode Correcting {sort_path} -> {bc_path}");
        ibu_command::correct::run_with_prebuilt_whitelist(&barcode_args, whitelist)?;

        eprintln!(">> Removing uncorrected file: {sort_path}");
        std::fs::remove_file(&sort_path)?;

        sort_path = bc_path.replace(".barcode.ibu", ".barcode.sort.ibu");
        eprintln!(">> Sorting corrected file: {bc_path} -> {sort_path}");

        let sort_args = ArgsSort::from_wf_path(&bc_path, &sort_path, 1);
        ibu_command::sort::run(&sort_args)?;

        eprintln!(">> Removing unsorted file: {bc_path}");
        std::fs::remove_file(&bc_path)?;
    }

    if !wf_args.skip_umi {
        let umi_path = sort_path.replace(".sort.ibu", ".umi.ibu");
        let umi_args = ArgsUmi::from_wf_path(&sort_path, &umi_path);

        eprintln!(">> UMI Correcting {sort_path} -> {umi_path}");
        ibu_command::umi::run(&umi_args)?;

        eprintln!(">> Removing uncorrected file: {sort_path}");
        std::fs::remove_file(&sort_path)?;

        sort_path = umi_path.replace(".umi.ibu", ".umi.sort.ibu");
        eprintln!(">> Sorting corrected file: {umi_path} -> {sort_path}");

        let sort_args = ArgsSort::from_wf_path(&umi_path, &sort_path, 1);
        ibu_command::sort::run(&sort_args)?;

        eprintln!(">> Removing unsorted file: {umi_path}");
        std::fs::remove_file(&umi_path)?;
    }

    let feature_path = format!("{prefix}.features.tsv");
    let count_path = sort_path.replace(".sort.ibu", ".counts.tsv");
    let count_args = ArgsCount::from_wf_path(&sort_path, &count_path, &feature_path, 1);

    eprintln!(">> Counting {sort_path} -> {count_path}");
    ibu_command::count::run(&count_args)?;

    Ok(())
}
