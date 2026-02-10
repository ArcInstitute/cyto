use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

use anyhow::{Result, bail};
use log::{debug, warn};

/// Validates output directory and handles force flag
pub fn validate_output_directory<P: AsRef<Path>>(outdir: P, force: bool) -> Result<()> {
    debug!("Validating output directory: {}", outdir.as_ref().display());
    if outdir.as_ref().exists() {
        if force {
            // Remove existing directory if force is enabled
            warn!("Removing existing directory: {}", outdir.as_ref().display());
            fs::remove_dir_all(outdir.as_ref())?;
        } else {
            bail!(
                "Output directory '{}' already exists. Use --force to overwrite.",
                outdir.as_ref().display()
            );
        }
    }

    Ok(())
}

/// Convenience function to open a file handle, creating directories as needed
pub fn open_file_handle<P: AsRef<Path>>(output_path: P) -> Result<Box<dyn Write + Send>> {
    // Create parent directories if they don't exist
    if let Some(parent) = output_path.as_ref().parent()
        && !parent.exists()
    {
        debug!("Creating parent directories for {}", parent.display());
        fs::create_dir_all(parent)?;
    }
    debug!("Opening file handle for {}", output_path.as_ref().display());
    let buffer = File::create(output_path).map(BufWriter::new)?;
    Ok(Box::new(buffer))
}

pub fn write_features<'a, P: AsRef<Path>, F: crate::FeatureWriter<'a>>(
    outdir: P,
    collection: &'a F,
) -> Result<()> {
    // Designate the output path
    let output_path = outdir.as_ref().join("metadata").join("features.tsv");

    // Open the output file
    let output_handle = open_file_handle(&output_path)?;

    // Write the features to the output file
    debug!("Saving features to: {}", output_path.display());
    collection.write_to(output_handle)?;

    Ok(())
}
