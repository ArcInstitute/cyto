use clap::Parser;

#[derive(Parser)]
#[clap(next_help_heading = "Output Options")]
pub struct Output {
    #[clap(short = 'o', long, default_value = "./scmap_out")]
    pub prefix: String,
    #[clap(short = 'H', long)]
    pub with_header: bool,
}
