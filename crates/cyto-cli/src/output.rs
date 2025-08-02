use clap::Parser;

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Output Options")]
pub struct ArgsOutput {
    /// Output directory path
    #[clap(short = 'o', long, default_value = "./cyto_out")]
    pub outdir: String,
    #[clap(short = 'H', long)]
    pub with_header: bool,
    /// Force overwrite of existing output directory
    #[clap(short = 'f', long)]
    pub force: bool,
}
