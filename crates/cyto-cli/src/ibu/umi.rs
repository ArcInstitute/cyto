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
            },
        }
    }
}

#[derive(Parser, Debug)]
pub struct OptionsCorrect {
    /// Output file to write to [default=stdout]
    #[clap(short, long)]
    pub output: Option<String>,

    /// Output file to write statistics to [default=stderr]
    ///
    /// Will output as json
    #[clap(short, long)]
    pub log: Option<String>,
}
