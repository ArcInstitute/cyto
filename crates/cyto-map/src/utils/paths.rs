use anyhow::Result;
use cyto_core::mappers::ProbeMapper;
use log::debug;

pub fn build_filepath(outdir: &str, name: Option<&str>) -> String {
    if let Some(name) = name {
        format!("{outdir}/ibu/{name}.ibu")
    } else {
        format!("{outdir}/ibu/output.ibu")
    }
}

pub fn build_filepaths(outdir: &str, probe_mapper: &ProbeMapper) -> Result<Vec<String>> {
    let mut filepaths = Vec::new();
    for aid in 0..probe_mapper.index_to_alias.num_unique_aliases() {
        let alias_str = probe_mapper.index_to_alias.alias_map[&aid].name_str()?;
        let filepath = build_filepath(outdir, Some(alias_str));
        filepaths.push(filepath);
    }
    Ok(filepaths)
}

pub fn delete_empty_path(filepath: &str) -> Result<(), std::io::Error> {
    if let Ok(metadata) = std::fs::metadata(filepath) {
        // If the file only contains a header, delete it
        if metadata.len() == ibu::SIZE_HEADER as u64 {
            debug!("Removing empty IBU file: {}", filepath);
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
