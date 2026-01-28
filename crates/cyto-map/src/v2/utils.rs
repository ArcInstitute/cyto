use std::path::{Path, PathBuf};

use anyhow::Result;
use cyto_io::open_file_handle;

use crate::v2::{Bijection, BoxedWriter, ResolvedGeometry};

pub fn build_filepath<P: AsRef<Path>>(outdir: P, name: Option<&str>) -> PathBuf {
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
    outdir: P,
    geometry: &ResolvedGeometry,
    bijection: &Bijection<String>,
) -> Result<Vec<BoxedWriter>> {
    let header = ibu::Header::new(
        geometry.get_barcode_length()? as u32,
        geometry.get_umi_length()? as u32,
    );
    let filepaths = build_filepaths(outdir, &bijection)?;
    let mut writers = Vec::default();
    for path in filepaths {
        let mut handle = open_file_handle(path)?;
        handle.write_all(header.as_bytes())?;
        handle.flush()?;
        writers.push(handle);
    }
    Ok(writers)
}
