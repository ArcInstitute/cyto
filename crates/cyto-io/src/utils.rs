use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write, stderr, stdin, stdout},
    path::Path,
};

use anyhow::{Context, Result};
use log::debug;

use crate::open_file_handle;

pub fn match_input<P: AsRef<Path>>(filepath: Option<P>) -> Result<Box<dyn Read + Send>> {
    if let Some(ref filepath) = filepath {
        debug!("Opening filepath: {}", filepath.as_ref().display());
        let handle = File::open(filepath).map(BufReader::new).context(format!(
            "Failed to open file for reading: {}",
            filepath.as_ref().display()
        ))?;
        Ok(Box::new(handle))
    } else {
        debug!("Reading from stdin");
        let handle = BufReader::new(stdin());
        Ok(Box::new(handle))
    }
}

pub fn match_input_transparent<P: AsRef<Path>>(
    filepath: Option<P>,
) -> Result<Box<dyn Read + Send>> {
    let handle = match_input(filepath)?;
    let (pass, comp) = niffler::send::get_reader(handle)?;
    match comp {
        niffler::send::compression::Format::No => {}
        _ => {
            debug!("Using transparent decompression for: {comp:?}");
        }
    }
    Ok(Box::new(pass))
}

pub fn match_output<P: AsRef<Path>>(filepath: Option<P>) -> Result<Box<dyn Write + Send>> {
    if let Some(ref filepath) = filepath {
        open_file_handle(filepath)
    } else {
        debug!("Opening stdout for writing");
        let handle = BufWriter::new(stdout());
        Ok(Box::new(handle))
    }
}

pub fn match_output_stderr<P: AsRef<Path>>(filepath: Option<P>) -> Result<Box<dyn Write + Send>> {
    if let Some(ref filepath) = filepath {
        open_file_handle(filepath)
    } else {
        debug!("Opening stdout for writing");
        let handle = BufWriter::new(stderr());
        Ok(Box::new(handle))
    }
}
