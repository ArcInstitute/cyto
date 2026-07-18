//! HTML QC report generation for finished cyto output directories.
//!
//! `cyto summary <dir>...` discovers sample directories, aggregates the per-stage
//! stat files each already wrote (`stats/mapping_*.json`, `stats/reads/*.zst`,
//! `stats/umi/*.json`, `stats/assignments/*.json`, `.timings.tsv`), and renders a
//! self-contained, sample-prefixed HTML report per sample -- plus a master index
//! when several samples are present -- alongside machine-readable JSON sidecars.
//!
//! Collection is best-effort: missing stat files degrade individual panels
//! rather than failing the report (see [`collect`]).

mod collect;
mod model;
mod render;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use cyto_cli::ArgsSummary;
use log::info;

pub use model::SampleReport;

/// Sanitize a string for use as a filename component.
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The shared parent directory of all samples, when they have one.
fn common_parent(samples: &[PathBuf]) -> Option<PathBuf> {
    let mut parents = samples.iter().filter_map(|p| p.parent());
    let first = parents.next()?;
    if parents.all(|p| p == first) {
        Some(first.to_path_buf())
    } else {
        None
    }
}

/// Entry point for `cyto summary`.
pub fn run(args: &ArgsSummary) -> Result<()> {
    let samples = collect::discover_samples(&args.inputs)?;
    if samples.is_empty() {
        bail!(
            "No cyto sample directories found in the provided input(s). \
             Point at a finished output directory (containing `stats/`) or a parent of several."
        );
    }
    info!("Found {} sample director(y/ies)", samples.len());

    let parent = common_parent(&samples);
    let outdir = args.outdir.clone().unwrap_or_else(|| {
        parent
            .clone()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cyto_reports")
    });
    std::fs::create_dir_all(&outdir)
        .with_context(|| format!("Unable to create report directory {}", outdir.display()))?;

    let run_name = args.run_name.clone().unwrap_or_else(|| {
        parent.as_ref().and_then(|p| p.file_name()).map_or_else(
            || "cyto_run".to_string(),
            |s| s.to_string_lossy().to_string(),
        )
    });

    let mut reports: Vec<(SampleReport, String)> = Vec::new();
    for sample_dir in &samples {
        let report = collect::collect_sample(sample_dir, args.rank_points);
        let stem = sanitize(&report.sample);
        let html_name = format!("{stem}.cyto-report.html");
        write_string(&outdir.join(&html_name), &render::render_sample(&report))?;
        if !args.no_json {
            write_json(&outdir.join(format!("{stem}.cyto-report.json")), &report)?;
        }
        info!("Wrote report for '{}' -> {}", report.sample, html_name);
        reports.push((report, html_name));
    }

    if reports.is_empty() {
        bail!("No reports could be generated from the provided input(s)");
    }

    if reports.len() > 1 && !args.no_master {
        let refs: Vec<(&SampleReport, String)> =
            reports.iter().map(|(r, h)| (r, h.clone())).collect();
        let master_name = format!("{}.cyto-report.html", sanitize(&run_name));
        write_string(
            &outdir.join(&master_name),
            &render::render_master(&run_name, &refs),
        )?;
        info!("Wrote master report -> {master_name}");
        println!("{}", outdir.join(&master_name).display());
    } else {
        println!("{}", outdir.join(&reports[0].1).display());
    }

    info!(
        "Done. {} report(s) written to {}",
        reports.len(),
        outdir.display()
    );
    Ok(())
}

fn write_string(path: &Path, contents: &str) -> Result<()> {
    std::fs::write(path, contents).with_context(|| format!("Unable to write {}", path.display()))
}

fn write_json(path: &Path, report: &SampleReport) -> Result<()> {
    let file = std::fs::File::create(path)
        .with_context(|| format!("Unable to create {}", path.display()))?;
    serde_json::to_writer_pretty(std::io::BufWriter::new(file), report)?;
    Ok(())
}
