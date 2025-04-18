use anyhow::Result;
use cyto_cli::ArgsView;
use cyto_core::PairedReader;
use cyto_io::match_output;

pub fn run(args: &ArgsView) -> Result<()> {
    let mut writer = match_output(args.options.output.as_ref())?;
    let mut reader = match (&args.input.r1, &args.input.r2) {
        (Some(r1), Some(r2)) => PairedReader::new(r1, r2)?,
        _ => unreachable!(),
    };
    reader.append_to(&mut writer, args.geometry.barcode, args.geometry.umi)?;
    Ok(())
}
