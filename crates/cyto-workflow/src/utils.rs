use std::io::Write;
use std::path::Path;
use std::process::Command;

use anyhow::bail;
use anyhow::{Context, Result};
use cyto_cli::{
    ibu::{ArgsBarcode, ArgsCount, ArgsSort, ArgsUmi},
    workflow::{ArgsWorkflow, WorkflowMode},
};
use cyto_ibu_barcode_correct::Whitelist;
use glob::glob;
use log::{debug, error, info, warn};
use tempfile::tempdir;

use crate::gex::DEFAULT_OUTPUT_BASENAME;

// Embed a python script to convert to mtx
const MTX_TO_H5AD_SCRIPT: &str = include_str!("../../../scripts/mtx_to_h5ad.py");

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
        .context(format!("Expected path ({ibu_path}) to end with .ibu"))?;
    let base_ibu_path = Path::new(base_ibu)
        .file_name()
        .context("Expected file name")?
        .to_str()
        .context("Expected valid UTF8")?;
    Ok(base_ibu_path)
}

fn convert_to_h5ad<P: AsRef<Path>>(count_path: P) -> Result<()> {
    info!(
        "Converting MTX {} -> {}.h5ad",
        count_path.as_ref().display(),
        count_path.as_ref().display()
    );

    // Create a temporary directory for the script
    let temp_dir = tempdir()?;

    // Write the script into the temporary directory
    let script_path = temp_dir.path().join("convert_script.py");
    {
        let mut file = std::fs::File::create(&script_path)?;
        file.write_all(MTX_TO_H5AD_SCRIPT.as_bytes())?;
        file.sync_all()?;
    }

    let chmod_output = Command::new("chmod").arg("+x").arg(&script_path).output()?;
    if !chmod_output.status.success() {
        error!("Unable to make h5ad conversion executable");
        bail!("Unable to make h5ad conversion executable");
    }

    let output = Command::new(&format!("{}", script_path.display()))
        .arg(count_path.as_ref().display().to_string())
        .arg(format!("{}.h5ad", count_path.as_ref().display()))
        .output()?;
    if output.status.success() {
        debug!(
            "Successfully converted {} to h5ad",
            count_path.as_ref().display()
        );
        debug!("Removing MTX directory");
        std::fs::remove_dir_all(&count_path).context(format!(
            "Unable to remove directory {}",
            count_path.as_ref().display()
        ))?;
    } else {
        error!(
            "Unable to run h5ad conversion for {}",
            count_path.as_ref().display()
        );
        error!("stdout: {}", std::str::from_utf8(&output.stdout)?);
        error!("stderr: {}", std::str::from_utf8(&output.stderr)?);
        bail!(
            "Unable to convert {} to h5ad",
            count_path.as_ref().display()
        );
    }

    Ok(())
}

fn filter_h5ad<P: AsRef<Path>>(count_path: P, mut keep_unfiltered: bool) -> Result<()> {
    let in_h5ad = count_path.as_ref().with_extension("h5ad");
    let out_h5ad = count_path.as_ref().with_extension("filt.h5ad");

    info!("Filtering h5ad file: {}", in_h5ad.display());
    let output = Command::new("cell-filter")
        .arg(&in_h5ad)
        .arg(&out_h5ad)
        .output()
        .context("Unable to run cell-filter")?;
    if !output.status.success() {
        error!("stdout: {}", std::str::from_utf8(&output.stdout)?);
        error!("stderr: {}", std::str::from_utf8(&output.stderr)?);
        bail!("Unable to filter {}", count_path.as_ref().display());
    }

    if out_h5ad.exists() {
        info!(
            "Successfully wrote filtered h5ad file: {}",
            out_h5ad.display()
        );
    } else {
        warn!(
            "Missing filtered h5ad file ({}); Likely due to insufficient barcodes or total UMIs",
            out_h5ad.display()
        );
        if !keep_unfiltered {
            warn!(
                "Skipping removal of unfiltered h5ad file: {}",
                in_h5ad.display()
            );
        }
        keep_unfiltered = true;
    }

    if !keep_unfiltered {
        // Remove the original h5ad file
        debug!("Removing unfiltered h5ad file: {}", in_h5ad.display());
        std::fs::remove_file(&in_h5ad).context(format!(
            "Unable to remove original h5ad file: {}",
            in_h5ad.display()
        ))?;
    }

    Ok(())
}

pub fn assign_guides<P: AsRef<Path>>(
    count_path: P,
    assignment_outdir: P,
    basename: &str,
) -> Result<()> {
    let in_h5ad = count_path.as_ref().with_extension("h5ad");
    let out_tsv = assignment_outdir
        .as_ref()
        .join(format!("{}.assignments.tsv", basename));

    info!(
        "Assigning CRISPR guide identities to: {}",
        in_h5ad.display()
    );
    let output = Command::new("geomux")
        .arg(&in_h5ad)
        .arg(&out_tsv)
        .output()
        .context("Unable to run geomux")?;
    if !output.status.success() {
        let stderr_str = std::str::from_utf8(&output.stderr)?;
        if stderr_str.contains("No guides passed the cell threshold") {
            warn!("No guides passed the cell threshold: {}", in_h5ad.display());
        } else {
            error!("stdout: {}", std::str::from_utf8(&output.stdout)?);
            error!("stderr: {}", std::str::from_utf8(&output.stderr)?);
            bail!(
                "Unable to assign guides to {}",
                count_path.as_ref().display()
            );
        }
    }

    if out_tsv.exists() {
        info!("Guide assignments written to {}", out_tsv.display());
    } else {
        warn!(
            "No guide assignments found for {}; Likely due to insufficient UMIs for barcodes",
            count_path.as_ref().display()
        );
    }

    Ok(())
}

pub fn ibu_steps<P: AsRef<Path>>(
    ibu_path: &str,
    outdir: P,
    wf_args: &ArgsWorkflow,
    whitelist: Option<Whitelist>,
    wf_mode: WorkflowMode,
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
            .join(format!("{base_ibu_path}.barcode.json"));

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
            .join(format!("{base_ibu_path}.umi.json"));

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

    // Locate the expected feature path
    let feature_path = outdir.as_ref().join("metadata").join("features.tsv");
    // Build the expected count path
    let count_path = if wf_args.mtx() {
        outdir.as_ref().join("counts").join(base_ibu_path)
    } else {
        outdir
            .as_ref()
            .join("counts")
            .join(format!("{base_ibu_path}.counts.tsv"))
    };
    // Create the argument struct
    let count_args = ArgsCount::from_wf_path(
        &sort_path,
        &count_path,
        &feature_path,
        1,
        wf_args.mtx(),
        if base_ibu_path == DEFAULT_OUTPUT_BASENAME {
            None
        } else {
            Some(base_ibu_path.to_string())
        },
    );

    // Run the counting step
    info!("Counting {sort_path} -> {}", count_path.display());
    cyto_ibu_count::run(&count_args)?;

    // Convert to h5ad if required
    if wf_args.to_h5ad() {
        convert_to_h5ad(&count_path)?;

        match wf_mode {
            WorkflowMode::Gex => {
                if !wf_args.no_filter {
                    filter_h5ad(&count_path, wf_args.keep_unfiltered)?;
                }
            }
            WorkflowMode::Crispr => {
                if !wf_args.skip_assignment {
                    let assignment_outdir = outdir.as_ref().join("assignments");
                    std::fs::create_dir_all(&assignment_outdir)
                        .context("Unable to build assignments output directory")?;
                    assign_guides(&count_path, &assignment_outdir, base_ibu_path)?;
                }
            }
        }
    }

    Ok(())
}
