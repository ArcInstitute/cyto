use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use anyhow::bail;
use anyhow::{Context, Result};
use cyto_cli::ibu::ArgsReads;
use cyto_cli::workflow::{
    ArgsGeomux, CrisprMappingCommand, GexMappingCommand, VERSION_CELL_FILTER, VERSION_GEOMUX,
    VERSION_PYCYTO, uvx_command,
};
use cyto_cli::{
    ibu::{ArgsCount, ArgsSort, ArgsUmi},
    workflow::{ArgsWorkflow, WorkflowMode},
};
use glob::glob;
use log::{debug, error, info, warn};

use crate::gex::DEFAULT_OUTPUT_BASENAME;
use crate::timing::{Module, ModuleTiming};

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

    let output = uvx_command("pycyto", VERSION_PYCYTO)
        .arg("convert")
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

/// Build the `cell-filter` invocation, routed through `uvx_command` so it runs in
/// an ephemeral, isolated environment at the pinned version.
///
/// `pycyto convert` (anndata >= 0.13) writes the barcode `obs` index as a nullable
/// string; cell-filter's older anndata (0.12.x) refuses to write it back unless
/// `ANNDATA_ALLOW_WRITE_NULLABLE_STRINGS=1`, else GEX filtering dies with a `RuntimeError`.
fn cell_filter_command(in_h5ad: &Path, out_h5ad: &Path, logfile: &Path) -> Command {
    let mut command = uvx_command("cell-filter", VERSION_CELL_FILTER);
    command
        .arg(in_h5ad)
        .arg(out_h5ad)
        .arg("--logfile")
        .arg(logfile)
        .env("ANNDATA_ALLOW_WRITE_NULLABLE_STRINGS", "1");
    command
}

fn filter_h5ad<P: AsRef<Path>>(
    count_path: P,
    stats_outdir: P,
    basename: &str,
    mut keep_unfiltered: bool,
) -> Result<()> {
    let in_h5ad = count_path.as_ref().with_extension("h5ad");
    let out_h5ad = count_path.as_ref().with_extension("filt.h5ad");
    let logfile = stats_outdir.as_ref().join(format!("{basename}.log"));

    info!("Filtering h5ad file: {}", in_h5ad.display());
    let output = cell_filter_command(&in_h5ad, &out_h5ad, &logfile)
        .output()
        .context("Unable to run cell-filter")?;

    if !output.status.success() {
        let stderr_str = std::str::from_utf8(&output.stderr)?;
        if stderr_str.contains("IndexError: index 1 is out of bounds") {
            // Allows the process to continue and will warn the user that this barcode has failed to filter
            // TODO: "Handle the error case in cell-filter ")
            warn!("Unable to filter {}", in_h5ad.display());
        } else {
            error!("stdout: {}", std::str::from_utf8(&output.stdout)?);
            error!("stderr: {}", std::str::from_utf8(&output.stderr)?);
            bail!("Unable to filter {}", count_path.as_ref().display());
        }
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
    stats_outdir: P,
    basename: &str,
    geomux_args: ArgsGeomux,
    threads: usize,
) -> Result<()> {
    let in_h5ad = count_path.as_ref().with_extension("h5ad");
    let out_tsv = assignment_outdir
        .as_ref()
        .join(format!("{basename}.assignments.tsv"));
    let stats_json = stats_outdir.as_ref().join(format!("{basename}.json"));

    info!(
        "Assigning CRISPR guide identities to: {}",
        in_h5ad.display()
    );

    let mut geomux_args_vec = vec![
        format!("{}", in_h5ad.display()),
        format!("{}", out_tsv.display()),
        "--stats".to_string(),
        format!("{}", stats_json.display()),
        "--min-umi-cells".to_string(),
        format!("{}", geomux_args.min_umi_cells()),
        "--min-umi-guides".to_string(),
        format!("{}", geomux_args.geomux_min_umi_guides),
        "--fdr-threshold".to_string(),
        format!("{}", geomux_args.geomux_fdr_threshold),
        "--method".to_string(),
        format!("{}", geomux_args.geomux_mode),
        "--n-jobs".to_string(),
        format!("{}", threads),
    ];
    if let Some(lor_threshold) = geomux_args.geomux_log_odds_ratio {
        geomux_args_vec.push("--lor-threshold".to_string());
        geomux_args_vec.push(format!("{lor_threshold}"));
    }
    let output = uvx_command("geomux", VERSION_GEOMUX)
        .args(&geomux_args_vec)
        .output()
        .context("Unable to run geomux")?;

    if !output.status.success() {
        let stderr_str = std::str::from_utf8(&output.stderr)?;
        if stderr_str.contains("No guides passed the cell threshold") {
            warn!("No guides passed the cell threshold: {}", in_h5ad.display());
        } else if stderr_str.contains("No cells passed the UMI threshold") {
            warn!("No cells passed the UMI threshold: {}", in_h5ad.display());
        } else if stderr_str.contains("No valid cell-guide pairs found") {
            warn!("No valid cell-guide pairs found: {}", in_h5ad.display());
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
    wf_mode: WorkflowMode,
    geomux_args: Option<ArgsGeomux>,
    threads: usize,
) -> Result<Vec<ModuleTiming>> {
    let mut timings = Vec::new();

    let base_ibu_path = strip_ibu_basename(ibu_path)?;
    let mut sort_path = ibu_path.replace(".ibu", ".sort.ibu");

    let sort_args = ArgsSort::from_wf_path(
        ibu_path,
        &sort_path,
        wf_args.sort_in_memory,
        wf_args.memory_limit.clone(),
        threads,
    );

    info!("Sorting {ibu_path} -> {sort_path}");
    let start = Instant::now();
    cyto_ibu_sort::run(&sort_args)?;
    let elapsed = start.elapsed();
    timings.push(ModuleTiming::new(
        base_ibu_path,
        Module::InitialSort,
        elapsed,
    ));

    debug!("Removing unsorted file: {ibu_path}");
    std::fs::remove_file(ibu_path)?;

    if !wf_args.skip_umi {
        let umi_path = sort_path.replace(".sort.ibu", ".umi.ibu");
        let umi_log = outdir
            .as_ref()
            .join("stats")
            .join("umi")
            .join(format!("{base_ibu_path}.umi.json"));

        let umi_args = ArgsUmi::from_wf_path(&sort_path, &umi_path, umi_log, threads);

        info!("UMI Correcting {sort_path} -> {umi_path}");
        let start = Instant::now();
        cyto_ibu_umi_correct::run(&umi_args)?;
        let elapsed = start.elapsed();
        timings.push(ModuleTiming::new(
            base_ibu_path,
            Module::UmiCorrection,
            elapsed,
        ));

        debug!("Removing uncorrected file: {sort_path}");
        std::fs::remove_file(&sort_path)?;

        sort_path = umi_path.clone();
    }

    if !wf_args.skip_reads {
        let reads_path = outdir
            .as_ref()
            .join("stats")
            .join("reads")
            .join(format!("{base_ibu_path}.reads.tsv.zst"));
        info!(
            "Barcode-level UMI and Reads counting for {sort_path} -> {}",
            reads_path.display()
        );
        let reads_args = ArgsReads::from_wf_path(&sort_path, &reads_path);

        let start = Instant::now();
        cyto_ibu_reads::run(&reads_args)?;
        let elapsed = start.elapsed();
        timings.push(ModuleTiming::new(base_ibu_path, Module::ReadsDump, elapsed));
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
    let start = Instant::now();
    cyto_ibu_count::run(&count_args)?;
    let elapsed = start.elapsed();
    timings.push(ModuleTiming::new(base_ibu_path, Module::Counting, elapsed));

    if !wf_args.keep_ibu {
        debug!("Removing IBU file: {sort_path}");
        std::fs::remove_file(&sort_path).context("Unable to remove IBU file")?;
    }

    // Convert to h5ad if required
    if wf_args.to_h5ad() {
        let start = Instant::now();
        convert_to_h5ad(&count_path)?;
        let elapsed = start.elapsed();
        timings.push(ModuleTiming::new(
            base_ibu_path,
            Module::ConversionH5ad,
            elapsed,
        ));

        match wf_mode {
            WorkflowMode::Gex => {
                if !wf_args.no_filter {
                    let filter_stats_outdir = outdir.as_ref().join("stats").join("filtering");
                    std::fs::create_dir_all(&filter_stats_outdir)
                        .context("Unable to build filter stats output directory")?;

                    let start = Instant::now();
                    filter_h5ad(
                        &count_path,
                        &filter_stats_outdir,
                        base_ibu_path,
                        wf_args.keep_unfiltered,
                    )?;
                    let elapsed = start.elapsed();
                    timings.push(ModuleTiming::new(
                        base_ibu_path,
                        Module::DropletFiltering,
                        elapsed,
                    ));
                }
            }
            WorkflowMode::Crispr => {
                if !wf_args.skip_assignment {
                    let assignment_outdir = outdir.as_ref().join("assignments");
                    let assignment_stats_outdir = outdir.as_ref().join("stats").join("assignments");
                    std::fs::create_dir_all(&assignment_outdir)
                        .context("Unable to build assignments output directory")?;
                    std::fs::create_dir_all(&assignment_stats_outdir)
                        .context("Unable to build assignments stats output directory")?;
                    let Some(geomux_args) = geomux_args else {
                        bail!("Expected geomux arguments")
                    };

                    let start = Instant::now();
                    assign_guides(
                        &count_path,
                        &assignment_outdir,
                        &assignment_stats_outdir,
                        base_ibu_path,
                        geomux_args,
                        threads,
                    )?;
                    let elapsed = start.elapsed();
                    timings.push(ModuleTiming::new(
                        base_ibu_path,
                        Module::GuideAssignment,
                        elapsed,
                    ));
                }
            }
        }
    }

    Ok(timings)
}

pub fn write_done_file<P: AsRef<Path>>(outdir: P, args: &RefWorkflowCommand) -> Result<()> {
    let done_file = outdir.as_ref().join(".done");
    let mut file = std::fs::File::create(&done_file)?;
    writeln!(&mut file, "{args:#?}")?;
    Ok(())
}

pub fn write_timings_file<P: AsRef<Path>>(outdir: P, timings: &[ModuleTiming]) -> Result<()> {
    let timings_file = outdir.as_ref().join(".timings.tsv");
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .has_headers(true)
        .from_path(&timings_file)?;

    for timing in timings {
        writer.serialize(timing)?;
    }
    writer.flush()?;

    Ok(())
}

pub fn remove_ibu_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    std::fs::remove_dir_all(path).context("Unable to remove IBU directory")?;
    Ok(())
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum RefWorkflowCommand<'a> {
    GexMapping(&'a GexMappingCommand),
    CrisprMapping(&'a CrisprMappingCommand),
}

#[cfg(test)]
mod tests {
    use super::cell_filter_command;
    use std::ffi::OsStr;
    use std::path::Path;

    /// Guard the nullable-string write opt-in; without it GEX filtering aborts.
    #[test]
    fn cell_filter_command_opts_into_nullable_string_writes() {
        let command = cell_filter_command(
            Path::new("in.h5ad"),
            Path::new("out.filt.h5ad"),
            Path::new("stats.log"),
        );
        let opt_in = command
            .get_envs()
            .find(|(key, _)| *key == OsStr::new("ANNDATA_ALLOW_WRITE_NULLABLE_STRINGS"));
        assert_eq!(
            opt_in,
            Some((
                OsStr::new("ANNDATA_ALLOW_WRITE_NULLABLE_STRINGS"),
                Some(OsStr::new("1")),
            )),
            "cell-filter must export ANNDATA_ALLOW_WRITE_NULLABLE_STRINGS=1 so it can \
             re-serialize the nullable-string barcode column written by pycyto convert",
        );
    }
}
