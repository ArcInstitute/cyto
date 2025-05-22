#[derive(clap::Parser, Debug)]
#[clap(next_help_heading = "IBU Input Options")]
pub struct IbuInput {
    /// Input ibu file [default=stdin]
    #[clap(short = 'i', long)]
    pub input: Option<String>,
}
impl IbuInput {
    pub fn from_path(path: &str) -> Self {
        Self {
            input: Some(path.to_string()),
        }
    }
}
