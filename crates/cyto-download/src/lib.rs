use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use cyto_cli::download::ArgsDownload;
use flate2::read::GzDecoder;
use log::info;
use tar::Archive;

const GITHUB_REPO: &str = "ArcInstitute/cyto";
const ASSET_NAME: &str = "cyto-resources.tar.gz";

/// Returns the default resource directory: `~/.cyto/`
pub fn default_resource_dir() -> Result<PathBuf> {
    let home = home_dir()?;
    Ok(home.join(".cyto"))
}

fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().context("could not determine home directory")
}

fn release_url(version: &str) -> String {
    format!("https://github.com/{GITHUB_REPO}/releases/download/cyto-{version}/{ASSET_NAME}")
}

pub fn run(args: &ArgsDownload, binary_version: &str) -> Result<()> {
    let version = args.version.as_deref().unwrap_or(binary_version);
    let output_dir = match &args.output {
        Some(dir) => dir.clone(),
        None => default_resource_dir()?,
    };

    if output_dir.exists() && !args.force {
        let marker = output_dir.join(".version");
        if marker.exists() {
            let existing = fs::read_to_string(&marker)?;
            if existing.trim() == version {
                info!(
                    "Resources for v{version} already present at {}",
                    output_dir.display()
                );
                info!("Use --force to re-download");
                return Ok(());
            }
        }
    }

    let url = args.url.clone().unwrap_or_else(|| release_url(version));
    info!("Downloading resources from: {url}");

    let response =
        reqwest::blocking::get(&url).with_context(|| format!("failed to download {url}"))?;

    if !response.status().is_success() {
        bail!(
            "failed to download resources: HTTP {} from {url}",
            response.status()
        );
    }

    let bytes = response.bytes().context("failed to read response body")?;

    info!("Extracting to: {}", output_dir.display());
    extract_tarball(&bytes, &output_dir)?;

    // Write version marker
    fs::write(output_dir.join(".version"), version)?;

    info!(
        "Resources for v{version} installed to {}",
        output_dir.display()
    );
    Ok(())
}

fn extract_tarball(data: &[u8], dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    let decoder = GzDecoder::new(Cursor::new(data));
    let mut archive = Archive::new(decoder);

    for entry in archive
        .entries()
        .context("failed to read tarball entries")?
    {
        let mut entry = entry.context("failed to read tarball entry")?;
        let path = entry.path().context("failed to read entry path")?;

        // Flatten: strip the top-level directory from the tarball path
        let stripped = path.components().skip(1).collect::<PathBuf>();

        if stripped.as_os_str().is_empty() {
            continue;
        }

        // Skip macOS AppleDouble resource fork files
        if stripped
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("._"))
        {
            continue;
        }

        let dest_path = dest.join(&stripped);
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        entry.unpack(&dest_path)?;
        info!("  {}", stripped.display());
    }

    Ok(())
}
