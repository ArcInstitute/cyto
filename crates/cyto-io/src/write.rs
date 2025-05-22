use anyhow::Result;
use cyto_core::{io::FeatureWriter, statistics::Statistics};
use std::{
    fs::File,
    io::{BufWriter, Write},
};

/// Convenience function to open a file handle
pub fn open_file_handle(output_path: &str) -> Result<Box<dyn Write + Send>> {
    let buffer = File::create(output_path).map(BufWriter::new)?;
    Ok(Box::new(buffer))
}

/// Writes the mapping statistics to a file
pub fn write_statistics(prefix: &str, statistics: &Statistics) -> Result<()> {
    // Designate the output path
    let output_path = format!("{prefix}.stats.json");

    // Open the output file
    let output_handle = open_file_handle(&output_path)?;

    // Write the statistics to the output file
    statistics.save_json(output_handle)?;

    Ok(())
}

pub fn write_features<'a, F: FeatureWriter<'a>>(prefix: &str, collection: &'a F) -> Result<()> {
    // Designate the output path
    let output_path = format!("{prefix}.features.tsv");

    // Open the output file
    let output_handle = open_file_handle(&output_path)?;

    // Write the features to the output file
    collection.write_to(output_handle)?;

    Ok(())
}
