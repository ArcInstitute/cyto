use super::IbuInput;

#[derive(clap::Parser)]
pub struct ArgsView {
    #[clap(flatten)]
    pub input: IbuInput,

    #[clap(flatten)]
    pub options: OptionsView,
}

#[derive(clap::Parser)]
pub struct OptionsView {
    #[clap(
        short,
        long,
        help = "Decode the contents of the IBU library (from 2bit)"
    )]
    pub decode: bool,

    #[clap(short, long, help = "Output file [default=stdout]")]
    pub output: Option<String>,
}
