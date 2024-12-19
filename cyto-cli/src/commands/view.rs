use anyhow::Result;
use cyto::PairedReader;

use crate::{cli::ArgsView, io::match_output};

pub fn run(args: &ArgsView) -> Result<()> {
    let mut reader = PairedReader::new(&args.input.r1, &args.input.r2)?;
    let writer = match_output(args.options.output.as_ref())?;
    reader.write_to(writer, args.geometry.barcode, args.geometry.umi)
}
