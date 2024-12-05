use std::{
    fs::File,
    io::{BufReader, Read},
};

use anyhow::Result;
use ibu::Reader;
use rayon::slice::ParallelSliceMut;

use super::utils::init_thread_pool;
use crate::{cli::ibu::ArgsSort, io::match_output};

fn pull_records<R: Read>(reader: Reader<R>) -> Result<Vec<ibu::Record>, ibu::BinaryFormatError> {
    reader.collect()
}

pub fn run(args: &ArgsSort) -> Result<()> {
    // Build IO handles
    let handle = File::open(&args.input.input).map(BufReader::new)?;
    let mut output = match_output(args.output.as_ref())?;

    // Initialize the reader and header
    let reader = Reader::new(handle)?;
    let header = reader.header();

    // Read in all records
    let mut records = pull_records(reader)?;

    // Sort the records
    if args.num_threads > 1 {
        init_thread_pool(args.num_threads)?;
        records.par_sort_unstable();
    } else {
        records.sort_unstable();
    }

    // Write the header
    header.write_bytes(&mut output)?;

    // Write the records
    for record in records {
        record.write_bytes(&mut output)?;
    }

    // Flush the output
    output.flush()?;

    Ok(())
}
