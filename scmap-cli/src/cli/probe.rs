use clap::Parser;

#[derive(Parser)]
#[clap(next_help_heading = "Probe Options")]
pub struct ProbeOptions {
    #[clap(short = 'p', long = "probes")]
    pub probes_filepath: Option<String>,
}
