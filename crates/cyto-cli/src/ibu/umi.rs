use std::path::Path;

use super::IbuInput;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct ArgsUmi {
    #[clap(flatten)]
    pub input: IbuInput,

    #[clap(flatten)]
    pub options: OptionsCorrect,
}
impl ArgsUmi {
    pub fn from_wf_path<P: AsRef<Path>>(sort_path: &str, umi_path: &str, log_path: P) -> Self {
        let input = IbuInput::from_path(sort_path);
        Self {
            input,
            options: OptionsCorrect {
                output: Some(umi_path.to_string()),
                log: Some(log_path.as_ref().display().to_string()),
                threads: 1,
            },
        }
    }
}

#[derive(Parser, Debug)]
pub struct OptionsCorrect {
    /// Output file to write to [default=stdout]
    #[clap(short, long)]
    pub output: Option<String>,

    /// Number of threads to use (0 for all available)
    #[clap(short = 'T', long, default_value_t = 1)]
    pub threads: usize,

    /// Output file to write statistics to [default=stderr]
    ///
    /// Will output as json
    #[clap(short, long)]
    pub log: Option<String>,
}
impl OptionsCorrect {
    pub fn threads(&self) -> usize {
        if self.threads == 0 {
            num_cpus::get()
        } else {
            self.threads.min(num_cpus::get())
        }
    }
}
