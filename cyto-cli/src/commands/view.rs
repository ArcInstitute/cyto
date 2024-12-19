use anyhow::Result;
use cyto::PairedReader;

use crate::{cli::ArgsView, io::match_output};

pub fn run(args: &ArgsView) -> Result<()> {
    let mut writer = match_output(args.options.output.as_ref())?;
    for (r1, r2) in args.input.iter_pairs() {
        let mut reader = PairedReader::new(&r1, &r2)?;
        reader.append_to(&mut writer, args.geometry.barcode, args.geometry.umi)?;
    }
    Ok(())
}
