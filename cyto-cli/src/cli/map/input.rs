use clap::Parser;

#[derive(Parser)]
#[clap(next_help_heading = "Paired Input Options")]
pub struct PairedInput {
    #[clap(short = 'i', long)]
    pub r1: String,
    #[clap(short = 'I', long)]
    pub r2: String,
}
