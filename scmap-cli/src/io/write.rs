use crate::cli::ArgsOutput;
use anyhow::Result;
use scmap::{
    io::{write_sparse_mtx, FeatureWriter},
    mappers::ProbeMapper,
    statistics::Statistics,
    BarcodeIndexCounter, ProbeBarcodeIndexCounter,
};
use std::{
    fs::File,
    io::{BufWriter, Write},
};

/// Checks compilation flags and arguments to determine if the output writing should be skipped
fn skip_if_needed(_args: &ArgsOutput) -> bool {
    #[cfg(feature = "benchmarking")]
    {
        if _args.skip_output {
            return true;
        }
    }
    false
}

/// Convenience function to open a file handle
fn open_file_handle(output_path: &str) -> Result<Box<dyn Write>> {
    let buffer = File::create(output_path).map(BufWriter::new)?;
    Ok(Box::new(buffer))
}

/// Writes the mapping statistics to a file
pub fn write_statistics(args: &ArgsOutput, statistics: &Statistics) -> Result<()> {
    if skip_if_needed(args) {
        return Ok(());
    }

    // Designate the output path
    let output_path = format!("{}.stats.json", args.prefix);

    // Open the output file
    let output_handle = open_file_handle(&output_path)?;

    // Write the statistics to the output file
    statistics.save_json(output_handle)?;

    Ok(())
}

pub fn write_features<'a, F: FeatureWriter<'a>>(
    args: &ArgsOutput,
    collection: &'a F,
) -> Result<()> {
    if skip_if_needed(args) {
        return Ok(());
    }

    // Designate the output path
    let output_path = format!("{}.features.tsv", args.prefix);

    // Open the output file
    let output_handle = open_file_handle(&output_path)?;

    // Write the features to the output file
    collection.write_to(output_handle)?;

    Ok(())
}

/// Writes each probe BUS matrix to a separate file
pub fn write_probe_matrices(
    args: &ArgsOutput,
    mapper: &ProbeMapper,
    counter: &ProbeBarcodeIndexCounter,
) -> Result<()> {
    if skip_if_needed(args) {
        return Ok(());
    }

    // Iterate over all probes and write the corresponding matrix
    for p_idx in counter.iter_probes() {
        // Get the alias of the probe
        let probe_alias = mapper.get_alias(*p_idx).unwrap().name_str()?;

        // Designate the output path
        let output_path = format!("{}.{}.mtx", args.prefix, probe_alias);

        // Get the bus counter for the probe
        let counts = counter.get_probe_counter(*p_idx).unwrap();

        // Write the BUS matrix to the output path
        impl_write_bus_matrix(&output_path, counts, args.with_header)?;
    }

    Ok(())
}

/// Writes a single BUS matrix to a file
pub fn write_bus_matrix(args: &ArgsOutput, counter: &BarcodeIndexCounter) -> Result<()> {
    // Only skip the matrix writing under certain compilation conditions and flags
    //
    // Ignored in production builds
    if skip_if_needed(args) {
        return Ok(());
    }

    // Designate the output path
    let output_path = format!("{}.mtx", args.prefix);

    // Write the BUS matrix to the output path
    impl_write_bus_matrix(&output_path, counter, args.with_header)
}

/// Internal writing function for a single BUS matrix
///
/// This function is used to write a single BUS matrix to a file
/// This function is fully parameterized and used by upstream convenience functions
/// which handle the arguments and parameterization
fn impl_write_bus_matrix(
    output_path: &str,
    counter: &BarcodeIndexCounter,
    with_header: bool,
) -> Result<()> {
    let mut output_handle = open_file_handle(output_path)?;
    write_sparse_mtx(&mut output_handle, counter, with_header)
}
