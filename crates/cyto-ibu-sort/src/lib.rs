use anyhow::Result;
use ext_sort::{ExternalSorter, ExternalSorterBuilder, LimitedBufferBuilder, RmpExternalChunk};
use ibu::Reader;

use cyto_cli::ibu::ArgsSort;
use cyto_io::{match_input, match_output};

/// Default chunk size for external sorting
///
/// 1M records = (3*u64 = 24 bytes * 1M = 24MB)
pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

fn pull_records<R: std::io::Read>(
    reader: Reader<R>,
) -> Result<Vec<ibu::Record>, ibu::BinaryFormatError> {
    reader.collect()
}

pub fn run(args: &ArgsSort) -> Result<()> {
    // Build IO handles
    let input = match_input(args.input.input.as_ref())?;
    let mut output = match_output(args.output.as_ref())?;

    // Initialize the reader and header
    let reader = Reader::new(input)?;
    let header = reader.header();

    if args.in_memory {
        let mut collection = pull_records(reader)?;
        collection.sort_unstable();

        // Write the header
        header.write_bytes(&mut output)?;

        // Write the records
        for record in collection {
            record.write_bytes(&mut output)?;
        }
    } else {
        // Define the external sorter
        let sorter: ExternalSorter<
            ibu::Record,
            ibu::BinaryFormatError,
            _,
            RmpExternalChunk<ibu::Record>,
        > = ExternalSorterBuilder::new()
            .with_buffer(LimitedBufferBuilder::new(DEFAULT_CHUNK_SIZE, true))
            .with_threads_number(args.num_threads)
            .build()?;

        // Sort the records using external sort
        let sorted = sorter.sort(reader)?;

        // Write the header
        header.write_bytes(&mut output)?;

        // Write the records
        for record in sorted {
            record?.write_bytes(&mut output)?;
        }
    }

    // Flush the output
    output.flush()?;

    Ok(())
}
