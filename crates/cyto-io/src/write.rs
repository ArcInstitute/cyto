use anyhow::Result;
use cyto_core::{io::FeatureWriter, statistics::Statistics};
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

/// Convenience function to open a file handle, creating directories as needed
pub fn open_file_handle(output_path: &str) -> Result<Box<dyn Write + Send>> {
    // Create parent directories if they don't exist
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent)?;
    }
    let buffer = File::create(output_path).map(BufWriter::new)?;
    Ok(Box::new(buffer))
}

/// Writes the mapping statistics to a file
pub fn write_statistics(outdir: &str, statistics: &Statistics) -> Result<()> {
    // Designate the output path
    let output_path = format!("{outdir}/stats/mapping.json");

    // Open the output file
    let output_handle = open_file_handle(&output_path)?;

    // Write the statistics to the output file
    statistics.save_json(output_handle)?;

    Ok(())
}

pub fn write_features<'a, F: FeatureWriter<'a>>(outdir: &str, collection: &'a F) -> Result<()> {
    // Designate the output path
    let output_path = format!("{outdir}/metadata/features.tsv");

    // Open the output file
    let output_handle = open_file_handle(&output_path)?;

    // Write the features to the output file
    collection.write_to(output_handle)?;

    Ok(())
}
