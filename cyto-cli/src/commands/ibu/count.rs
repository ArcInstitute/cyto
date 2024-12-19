use crate::{
    cli::ibu::ArgsCount,
    io::{match_input, match_output},
};
use anyhow::Result;
use cyto::deduplicate_umis;
use ibu::Reader;

pub fn run(args: &ArgsCount) -> Result<()> {
    let input = match_input(args.input.input.as_ref())?;

    let reader = Reader::new(input)?;
    let _header = reader.header();
    let counts = deduplicate_umis(reader)?;
    let output_handle = match_output(args.output.as_ref())?;
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .from_writer(output_handle);

    counts
        .iter_counts()
        .try_for_each(|count| -> Result<(), csv::Error> {
            writer.serialize(count)?;
            Ok(())
        })?;

    writer.flush()?;

    Ok(())
}
