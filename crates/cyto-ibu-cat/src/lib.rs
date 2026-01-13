use anyhow::{Result, bail};

use cyto_cli::ibu::ArgsCat;
use cyto_io::match_output;
use ibu::Writer;

pub fn run(args: &ArgsCat) -> Result<()> {
    // Build input handles
    let mut inputs = args
        .input
        .inputs
        .iter()
        .map(|input| ibu::Reader::from_path(input))
        .collect::<Result<Vec<_>, _>>()?;

    // Validate all headers are the same
    let mut og_header = None;
    for reader in &mut inputs {
        if let Some(og_header) = og_header {
            if og_header != reader.header() {
                bail!("IBU headers do not match!");
            }
        } else {
            og_header = Some(reader.header());
        }
    }
    let header = og_header.expect("No headers found... Something went wrong!");

    // Build output handles
    let output = match_output(args.output.as_ref())?;
    let mut writer = Writer::new(output, header)?;

    // Dump all records into the output
    for reader in &mut inputs {
        for record in reader {
            writer.write_record(&record?)?;
        }
    }

    // Flush the output
    writer.finish()?;

    Ok(())
}
