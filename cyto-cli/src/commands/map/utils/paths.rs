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
    probe_mapper
        .index_to_alias
        .alias_map
        .values()
        .map(|alias| -> Result<String> {
            let alias_str = alias.name_str()?;
            Ok(build_filepath(prefix, Some(alias_str)))
        })
        .collect()
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
