use clap::{
    builder::{
        styling::{AnsiColor, Effects},
        Styles,
    },
    Parser, Subcommand,
};

// Configures Clap v3-style help menu colors
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser)]
#[command(styles = STYLES)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Crispr(ArgsCrispr),
}

#[derive(Parser)]
pub struct ArgsCrispr {
    #[clap(flatten)]
    pub input: PairedInput,

    #[clap(flatten)]
    pub geometry: Geometry,

    #[clap(flatten)]
    pub crispr: CrisprOptions,

    #[clap(flatten)]
    pub probe: ProbeOptions,

    #[clap(flatten)]
    pub output: Output,
}

#[derive(Parser)]
#[clap(next_help_heading = "Paired Input Options")]
pub struct PairedInput {
    #[clap(short = 'i', long)]
    pub r1: String,
    #[clap(short = 'I', long)]
    pub r2: String,
}

#[derive(Parser)]
#[clap(next_help_heading = "Output Options")]
pub struct Output {
    #[clap(short = 'o', long, default_value = "./scmap_out")]
    pub prefix: String,
    #[clap(short = 'H', long)]
    pub with_header: bool,
}

#[derive(Parser)]
#[clap(next_help_heading = "Geometry Configuration")]
pub struct Geometry {
    #[clap(short = 'b', long, default_value = "16")]
    pub barcode: usize,
    #[clap(short = 'u', long, default_value = "12")]
    pub umi: usize,
}

#[derive(Parser)]
#[clap(next_help_heading = "CRISPR Options")]
pub struct CrisprOptions {
    #[clap(short = 'c', long = "guides")]
    pub guides_filepath: String,
    #[clap(short = 's', long, default_value = "26")]
    pub offset: usize,
}

#[derive(Parser)]
#[clap(next_help_heading = "Probe Options")]
pub struct ProbeOptions {
    #[clap(short = 'p', long = "probes")]
    pub probes_filepath: Option<String>,
}
