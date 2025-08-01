use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

use anyhow::{Result, bail};
use cyto_core::{io::FeatureWriter, statistics::Statistics};

/// Validates output directory and handles force flag
pub fn validate_output_directory(outdir: &str, force: bool) -> Result<()> {
    let path = Path::new(outdir);

    // make sure path is not `.`
    let cwd = std::env::current_dir()?;
    if path.canonicalize()? == cwd {
        bail!(
            "Output directory cannot resolve to current working directory: {}",
            cwd.display()
        );
    }

    if path.exists() {
        if !force {
            bail!(
                "Output directory '{}' already exists. Use --force to overwrite.",
                outdir
            );
        }
        // Remove existing directory if force is enabled
        fs::remove_dir_all(path)?;
    }

    Ok(())
}

/// Convenience function to open a file handle, creating directories as needed
pub fn open_file_handle<P: AsRef<Path>>(output_path: P) -> Result<Box<dyn Write + Send>> {
    // Create parent directories if they don't exist
    if let Some(parent) = output_path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    let buffer = File::create(output_path).map(BufWriter::new)?;
    Ok(Box::new(buffer))
}

/// Writes the mapping statistics to a file
pub fn write_statistics<P: AsRef<Path>>(outdir: P, statistics: &Statistics) -> Result<()> {
    // Designate the output path
    let output_path = outdir.as_ref().join("stats").join("mapping.json");

    // Open the output file
    let output_handle = open_file_handle(&output_path)?;

    // Write the statistics to the output file
    statistics.save_json(output_handle)?;

    Ok(())
}

pub fn write_features<'a, P: AsRef<Path>, F: FeatureWriter<'a>>(
    outdir: P,
    collection: &'a F,
) -> Result<()> {
    // Designate the output path
    let output_path = outdir.as_ref().join("metadata").join("features.tsv");

    // Open the output file
    let output_handle = open_file_handle(&output_path)?;

    // Write the features to the output file
    collection.write_to(output_handle)?;

    Ok(())
}
