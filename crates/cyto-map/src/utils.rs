use std::path::{Path, PathBuf};

use anyhow::Result;
use cyto_io::open_file_handle;
use log::debug;

use crate::{Bijection, BoxedWriter, ResolvedGeometry};

fn build_filepath<P: AsRef<Path>>(outdir: P, name: Option<&str>) -> PathBuf {
    outdir.as_ref().join(if let Some(name) = name {
        Path::new("ibu").join(format!("{name}.ibu"))
    } else {
        Path::new("ibu").join("output.ibu")
    })
}

pub fn build_filepaths<P: AsRef<Path>>(
    outdir: P,
    bijection: &Bijection<String>,
) -> Result<Vec<PathBuf>> {
    let mut filepaths = Vec::new();
    for idx in 0..bijection.len() {
        let alias_str = bijection
            .get_element(idx)
            .expect("Bijection was incorrectly built");
        let filepath = build_filepath(&outdir, Some(alias_str));
        filepaths.push(filepath);
    }
    Ok(filepaths)
}

pub fn initialize_output_ibus<P: AsRef<Path>>(
    paths: &[P],
    geometry: &ResolvedGeometry,
) -> Result<Vec<BoxedWriter>> {
    let header = ibu::Header::new(
        geometry.get_barcode_length()? as u32,
        geometry.get_umi_length()? as u32,
    );
    let mut writers = Vec::default();
    for path in paths {
        let mut handle = open_file_handle(path)?;
        handle.write_all(header.as_bytes())?;
        handle.flush()?;
        writers.push(handle);
    }
    Ok(writers)
}

/// Returns the number of records in an IBU file based on its byte size.
fn ibu_record_count(file_size: u64) -> u64 {
    file_size.saturating_sub(ibu::HEADER_SIZE as u64) / ibu::RECORD_SIZE as u64
}

/// Delete IBU files that fall below the minimum record threshold.
///
/// When `min_records` is 0, only truly empty files (header-only) are removed.
pub fn delete_sparse_ibus<P: AsRef<Path>>(
    filepaths: &[P],
    min_records: u64,
) -> Result<(), std::io::Error> {
    for filepath in filepaths {
        if let Ok(metadata) = std::fs::metadata(filepath) {
            let n_records = ibu_record_count(metadata.len());
            let below_threshold = if min_records == 0 {
                n_records == 0
            } else {
                n_records < min_records
            };
            if below_threshold {
                debug!(
                    "Removing IBU file with {n_records} records (min: {min_records}): {}",
                    filepath.as_ref().display()
                );
                std::fs::remove_file(filepath)?;
            }
        }
    }
    Ok(())
}
