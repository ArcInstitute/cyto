use clap::Parser;

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Output Options")]
pub struct ArgsOutput {
    #[clap(short = 'o', long, default_value = "./cyto_out")]
    pub prefix: String,
    #[clap(short = 'H', long)]
    pub with_header: bool,
    #[cfg(feature = "benchmarking")]
    #[clap(long)]
    pub skip_output: bool,
}
