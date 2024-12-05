#[derive(clap::Parser)]
#[clap(next_help_heading = "IBU Input Options")]
pub struct IbuInput {
    #[clap(short = 'i', long)]
    pub input: String,
}
