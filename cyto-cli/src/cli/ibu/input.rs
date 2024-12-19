#[derive(clap::Parser)]
#[clap(next_help_heading = "IBU Input Options")]
pub struct IbuInput {
    /// Input ibu file [default=stdin]
    #[clap(short = 'i', long)]
    pub input: Option<String>,
}
