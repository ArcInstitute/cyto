use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write, stdin, stdout},
    path::Path,
};

use anyhow::{Context, Result};

pub fn match_input<P: AsRef<Path>>(filepath: Option<P>) -> Result<Box<dyn Read + Send>> {
    if let Some(ref filepath) = filepath {
        let handle = File::open(filepath).map(BufReader::new).context(format!(
            "Failed to open file for reading: {}",
            filepath.as_ref().display()
        ))?;
        Ok(Box::new(handle))
    } else {
        let handle = BufReader::new(stdin());
        Ok(Box::new(handle))
    }
}

pub fn match_input_transparent<P: AsRef<Path>>(
    filepath: Option<P>,
) -> Result<Box<dyn Read + Send>> {
    let handle = match_input(filepath)?;
    let (pass, _comp) = niffler::send::get_reader(handle)?;
    Ok(Box::new(pass))
}

pub fn match_output<P: AsRef<Path>>(filepath: Option<P>) -> Result<Box<dyn Write + Send>> {
    if let Some(filepath) = filepath {
        let handle = File::create(filepath).map(BufWriter::new)?;
        Ok(Box::new(handle))
    } else {
        let handle = BufWriter::new(stdout());
        Ok(Box::new(handle))
    }
}
