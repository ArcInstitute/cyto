use clap::Parser;

#[derive(Parser, Debug)]
#[clap(next_help_heading = "Probe Options")]
pub struct ProbeOptions {
    #[clap(short = 'p', long = "probes")]
    pub probes_filepath: Option<String>,

    /// Only match probes whose alias matches the given regex
    ///
    /// i.e. "BC00[123]" will only match probes with aliases starting with "BC00" followed by a digit from 1 to 3.
    #[clap(short = 'r', long = "regex")]
    pub regex: Option<String>,
}
