use std::path::Path;

use anyhow::Result;
use anyhow::bail;
use glob::glob;

use cyto_cli::{
    ibu::{ArgsBarcode, ArgsCount, ArgsSort, ArgsUmi},
    workflow::ArgsWorkflow,
};
use cyto_ibu_barcode_correct::Whitelist;
use log::info;

pub fn identify_ibu_files<P: AsRef<Path>>(outdir: P) -> Result<Vec<String>> {
    let ibu_files = glob(&format!("{}/ibu/*.ibu", outdir.as_ref().display()))?
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

pub fn ibu_steps<P: AsRef<Path>>(
    ibu_path: &str,
    outdir: P,
    wf_args: &ArgsWorkflow,
    whitelist: Option<Whitelist>,
) -> Result<()> {
    let mut sort_path = ibu_path.replace(".ibu", ".sort.ibu");
    let sort_args = ArgsSort::from_wf_path(ibu_path, &sort_path, 1);

    info!("Sorting {ibu_path} -> {sort_path}");
    cyto_ibu_sort::run(&sort_args)?;

    info!("Removing unsorted file: {ibu_path}");
    std::fs::remove_file(ibu_path)?;

    if !wf_args.skip_barcode {
        let bc_path = sort_path.replace(".sort.ibu", ".barcode.ibu");
        let barcode_args = ArgsBarcode::from_wf_path(&sort_path, &bc_path, &wf_args.whitelist);
        let Some(whitelist) = whitelist else {
            bail!("Whitelist is required for barcode correction");
        };

        info!("Barcode Correcting {sort_path} -> {bc_path}");
        cyto_ibu_barcode_correct::run_with_prebuilt_whitelist(&barcode_args, whitelist)?;

        info!("Removing uncorrected file: {sort_path}");
        std::fs::remove_file(&sort_path)?;

        sort_path = bc_path.replace(".barcode.ibu", ".barcode.sort.ibu");
        info!("Sorting corrected file: {bc_path} -> {sort_path}");

        let sort_args = ArgsSort::from_wf_path(&bc_path, &sort_path, 1);
        cyto_ibu_sort::run(&sort_args)?;

        info!("Removing unsorted file: {bc_path}");
        std::fs::remove_file(&bc_path)?;
    }

    if !wf_args.skip_umi {
        let umi_path = sort_path.replace(".sort.ibu", ".umi.ibu");
        let umi_args = ArgsUmi::from_wf_path(&sort_path, &umi_path);

        info!("UMI Correcting {sort_path} -> {umi_path}");
        cyto_ibu_umi_correct::run(&umi_args)?;

        info!("Removing uncorrected file: {sort_path}");
        std::fs::remove_file(&sort_path)?;

        sort_path = umi_path.replace(".umi.ibu", ".umi.sort.ibu");
        info!("Sorting corrected file: {umi_path} -> {sort_path}");

        let sort_args = ArgsSort::from_wf_path(&umi_path, &sort_path, 1);
        cyto_ibu_sort::run(&sort_args)?;

        info!("Removing unsorted file: {umi_path}");
        std::fs::remove_file(&umi_path)?;
    }

    let feature_path = outdir.as_ref().join("metadata").join("features.tsv");

    // Extract the base name from the IBU file path for the counts directory
    let base_name = std::path::Path::new(&sort_path)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .replace(".sort.ibu", "");

    let count_path = outdir
        .as_ref()
        .join("counts")
        .join(format!("{base_name}.counts.tsv"));

    // Create the counts directory if it doesn't exist
    let counts_dir = outdir.as_ref().join("counts");
    std::fs::create_dir_all(counts_dir)?;

    let count_args = ArgsCount::from_wf_path(&sort_path, &count_path, &feature_path, 1);

    info!("Counting {sort_path} -> {}", count_path.display());
    cyto_ibu_count::run(&count_args)?;

    Ok(())
}
