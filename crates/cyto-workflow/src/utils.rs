use std::path::Path;

use anyhow::bail;
use anyhow::{Context, Result};
use cyto_cli::{
    ibu::{ArgsBarcode, ArgsCount, ArgsSort, ArgsUmi},
    workflow::ArgsWorkflow,
};
use cyto_ibu_barcode_correct::Whitelist;
use glob::glob;
use log::{debug, error, info};

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

fn strip_ibu_basename(ibu_path: &str) -> Result<&str> {
    let base_ibu = ibu_path
        .strip_suffix(".ibu")
        .context(format!("Expected path ({}) to end with .ibu", ibu_path))?;
    let base_ibu_path = Path::new(base_ibu)
        .file_name()
        .context("Expected file name")?
        .to_str()
        .context("Expected valid UTF8")?;
    Ok(base_ibu_path)
}

pub fn ibu_steps<P: AsRef<Path>>(
    ibu_path: &str,
    outdir: P,
    wf_args: &ArgsWorkflow,
    whitelist: Option<Whitelist>,
) -> Result<()> {
    let base_ibu_path = strip_ibu_basename(ibu_path)?;
    let mut sort_path = ibu_path.replace(".ibu", ".sort.ibu");
    let sort_args = ArgsSort::from_wf_path(ibu_path, &sort_path, 1);

    info!("Sorting {ibu_path} -> {sort_path}");
    cyto_ibu_sort::run(&sort_args)?;

    debug!("Removing unsorted file: {ibu_path}");
    std::fs::remove_file(ibu_path)?;

    if !wf_args.skip_barcode {
        let bc_path = sort_path.replace(".sort.ibu", ".barcode.ibu");
        let bc_log = outdir
            .as_ref()
            .join("stats")
            .join("barcode")
            .join(&format!("{}.barcode.json", base_ibu_path));

        let barcode_args =
            ArgsBarcode::from_wf_path(&sort_path, &bc_path, &wf_args.whitelist, bc_log);
        let Some(whitelist) = whitelist else {
            error!("Whitelist is required for barcode correction");
            bail!("Whitelist is required for barcode correction");
        };

        info!("Barcode Correcting {sort_path} -> {bc_path}");
        cyto_ibu_barcode_correct::run_with_prebuilt_whitelist(&barcode_args, whitelist)?;

        debug!("Removing uncorrected file: {sort_path}");
        std::fs::remove_file(&sort_path)?;

        sort_path = bc_path.replace(".barcode.ibu", ".barcode.sort.ibu");
        info!("Sorting barcode corrected file: {bc_path} -> {sort_path}");

        let sort_args = ArgsSort::from_wf_path(&bc_path, &sort_path, 1);
        cyto_ibu_sort::run(&sort_args)?;

        debug!("Removing unsorted file: {bc_path}");
        std::fs::remove_file(&bc_path)?;
    }

    if !wf_args.skip_umi {
        let umi_path = sort_path.replace(".sort.ibu", ".umi.ibu");
        let umi_log = outdir
            .as_ref()
            .join("stats")
            .join("umi")
            .join(&format!("{}.umi.json", base_ibu_path));

        let umi_args = ArgsUmi::from_wf_path(&sort_path, &umi_path, umi_log);

        info!("UMI Correcting {sort_path} -> {umi_path}");
        cyto_ibu_umi_correct::run(&umi_args)?;

        debug!("Removing uncorrected file: {sort_path}");
        std::fs::remove_file(&sort_path)?;

        sort_path = umi_path.replace(".umi.ibu", ".umi.sort.ibu");
        info!("Sorting UMI corrected file: {umi_path} -> {sort_path}");

        let sort_args = ArgsSort::from_wf_path(&umi_path, &sort_path, 1);
        cyto_ibu_sort::run(&sort_args)?;

        debug!("Removing unsorted file: {umi_path}");
        std::fs::remove_file(&umi_path)?;
    }

    // Perform counting
    {
        // Locate the expected feature path
        let feature_path = outdir.as_ref().join("metadata").join("features.tsv");
        // Build the expected count path
        let count_path = if wf_args.mtx {
            outdir
                .as_ref()
                .join("counts")
                .join(format!("{base_ibu_path}"))
        } else {
            outdir
                .as_ref()
                .join("counts")
                .join(format!("{base_ibu_path}.counts.tsv"))
        };
        // Create the argument struct
        let count_args =
            ArgsCount::from_wf_path(&sort_path, &count_path, &feature_path, 1, wf_args.mtx);

        // Run the counting step
        info!("Counting {sort_path} -> {}", count_path.display());
        cyto_ibu_count::run(&count_args)?;
    }

    Ok(())
}
