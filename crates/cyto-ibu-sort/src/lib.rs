use std::str::FromStr;

use anyhow::{Context, Result};
use bytesize::ByteSize;
use ext_sort::{ExternalSorter, ExternalSorterBuilder, LimitedBufferBuilder, RmpExternalChunk};
use ibu::{Reader, Writer};

use cyto_cli::ibu::ArgsSort;
use cyto_io::{match_input, match_output};
use log::{debug, error, trace};

/// Size of a single IBU record in bytes
const RECORD_SIZE: u64 = 24;

/// Default memory limit per sort operation (5GiB)
const DEFAULT_MEMORY_LIMIT: u64 = 5;

fn pull_records<R: std::io::Read>(reader: Reader<R>) -> Result<Vec<ibu::Record>, ibu::IbuError> {
    reader.collect()
}

pub fn run(args: &ArgsSort) -> Result<()> {
    // Build IO handles
    let input = match_input(args.input.input.as_ref())?;
    let output = match_output(args.output.as_ref())?;

    // Initialize the reader and header
    let reader = Reader::new(input)?;
    let header = reader.header();

    // Initialize the writer
    let mut writer = Writer::new(output, header)?;

    if args.in_memory {
        trace!(
            "Sorting in memory: {}",
            args.input.input.as_deref().unwrap_or("stdin")
        );
        let mut collection = pull_records(reader)?;
        collection.sort_unstable();

        writer.write_batch(&collection)?;
    } else {
        trace!(
            "Sorting externally: {}",
            args.input.input.as_deref().unwrap_or("stdin")
        );
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
            ibu::IbuError,
            LimitedBufferBuilder,
            RmpExternalChunk<ibu::Record>,
        > = ExternalSorterBuilder::new()
            .with_buffer(LimitedBufferBuilder::new(chunk_size, false))
            .with_threads_number(args.num_threads)
            .build()
            .context("Failed to build external sorter")?;

        // Sort the records using external sort
        let sorted = sorter.sort(reader).with_context(|| {
            error!(
                "Failed to sort with external sort for file: {}",
                args.input.input.as_deref().unwrap_or("stdin")
            );
            "Failed to sort with external sort for file"
        })?;

        // Write the records
        for result in sorted {
            let record = result.with_context(|| {
                error!(
                    "Failed to deserialize record for file: {}",
                    args.input.input.as_deref().unwrap_or("stdin")
                );
                "Failed to deserialize record"
            })?;
            writer.write_record(&record)?;
        }
    }

    // Flush the output
    writer.finish()?;

    Ok(())
}
