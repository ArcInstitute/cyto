use std::str::FromStr;

use anyhow::Result;
use bytesize::ByteSize;
use ext_sort::{ExternalSorter, ExternalSorterBuilder, LimitedBufferBuilder, RmpExternalChunk};
use ibu::Reader;

use cyto_cli::ibu::ArgsSort;
use cyto_io::{match_input, match_output};
use log::debug;

/// Size of a single IBU record in bytes
const RECORD_SIZE: u64 = 24;

/// Default memory limit per sort operation (5GiB)
const DEFAULT_MEMORY_LIMIT: u64 = 5;

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
        let memory_limit =
            ByteSize::from_str(&args.memory_limit).unwrap_or(ByteSize::gib(DEFAULT_MEMORY_LIMIT));
        let chunk_size = (memory_limit.as_u64() / RECORD_SIZE) as usize;

        debug!(
            "External sorting with {} memory limit ({} records/chunk, {} threads) for file {}",
            memory_limit,
            chunk_size,
            args.num_threads,
            args.input.input.as_deref().unwrap_or("stdin")
        );

        // Build the external sorter with count-limited buffer
        let sorter: ExternalSorter<
            ibu::Record,
            ibu::BinaryFormatError,
            LimitedBufferBuilder,
            RmpExternalChunk<ibu::Record>,
        > = ExternalSorterBuilder::new()
            .with_buffer(LimitedBufferBuilder::new(chunk_size, false))
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
