use std::path::PathBuf;

/// Arguments for the `summary` command.
///
/// Consumes one or more finished cyto output directories and renders
/// self-contained HTML QC reports (plus machine-readable JSON sidecars).
#[derive(clap::Parser, Debug)]
pub struct ArgsSummary {
    /// One or more finished cyto output directories.
    ///
    /// Each path may be a single sample directory (containing `stats/`), or a
    /// parent directory holding several sample directories -- in which case the
    /// sample directories are auto-discovered.
    #[clap(required = true, num_args = 1..)]
    pub inputs: Vec<PathBuf>,

    /// Directory to write reports into
    ///
    /// [default: `<common-parent>/cyto_reports`]
    #[clap(short, long)]
    pub outdir: Option<PathBuf>,

    /// Name used for the master report title and filename
    ///
    /// [default: the name of the common parent directory]
    #[clap(long)]
    pub run_name: Option<String>,

    /// Do not write the machine-readable JSON sidecar per sample
    #[clap(long)]
    pub no_json: bool,

    /// Do not write a master report even when multiple samples are present
    #[clap(long)]
    pub no_master: bool,

    /// Skip reading count matrices for genes/guides-detected metrics
    ///
    /// Reading the h5ad panels adds a pass over the count matrices; disable it
    /// for a faster, metrics-only report.
    #[clap(long)]
    pub no_features: bool,

    /// Number of points to sample for each barcode-rank plot polyline
    #[clap(long, default_value_t = 400)]
    pub rank_points: usize,
}
