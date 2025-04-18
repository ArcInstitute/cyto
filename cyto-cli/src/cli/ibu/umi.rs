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
    pub fn from_wf_path(sort_path: &str, umi_path: &str) -> Self {
        let input = IbuInput::from_path(sort_path);
        Self {
            input,
            options: OptionsCorrect {
                output: Some(umi_path.to_string()),
            },
        }
    }
}

#[derive(Parser, Debug)]
pub struct OptionsCorrect {
    /// Output file to write to [default=stdout]
    #[clap(short, long)]
    pub output: Option<String>,
}
