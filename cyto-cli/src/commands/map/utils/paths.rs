use std::{
    fs::{File, OpenOptions},
    io::BufWriter,
};

use anyhow::Result;
use cyto::mappers::ProbeMapper;

pub fn build_filepath(prefix: &str, name: Option<&str>) -> String {
    if let Some(name) = name {
        format!("{prefix}.{name}.ibu")
    } else {
        format!("{prefix}.ibu")
    }
}

pub fn build_filepaths(prefix: &str, probe_mapper: &ProbeMapper) -> Result<Vec<String>> {
    (0..probe_mapper.num_unique_aliases())
        .map(|index| -> Result<String> {
            let alias = probe_mapper
                .get_alias(index)
                .expect("Alias not found - `build_filepaths`");
            let alias_str = std::str::from_utf8(&alias.name)?;
            Ok(build_filepath(prefix, Some(alias_str)))
        })
        .collect()
}

pub fn open_handle(filepath: &str) -> Result<BufWriter<File>, std::io::Error> {
    File::create(filepath).map(BufWriter::new)
}

#[allow(dead_code)]
/// Used in binseq to reopen a handle for appending
pub fn reopen_handle(filepath: &str) -> Result<BufWriter<File>, std::io::Error> {
    OpenOptions::new()
        .append(true)
        .open(filepath)
        .map(BufWriter::new)
}

pub fn open_handles(filepaths: &[String]) -> Result<Vec<BufWriter<File>>, std::io::Error> {
    filepaths.iter().map(|path| open_handle(path)).collect()
}

pub fn delete_empty_path(filepath: &str) -> Result<(), std::io::Error> {
    if let Ok(metadata) = std::fs::metadata(filepath) {
        // If the file only contains a header, delete it
        if metadata.len() == ibu::SIZE_HEADER as u64 {
            std::fs::remove_file(filepath)?;
        }
    }
    Ok(())
}

pub fn delete_empty_paths(filepaths: &[String]) -> Result<(), std::io::Error> {
    filepaths
        .iter()
        .try_for_each(|path| delete_empty_path(path))
}
