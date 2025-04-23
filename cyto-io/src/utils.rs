use anyhow::Result;
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write, stdin, stdout},
};

pub fn match_input(filepath: Option<&String>) -> Result<Box<dyn Read + Send>> {
    if let Some(filepath) = filepath {
        let handle = File::open(filepath).map(BufReader::new)?;
        Ok(Box::new(handle))
    } else {
        let handle = BufReader::new(stdin());
        Ok(Box::new(handle))
    }
}

pub fn match_input_transparent(filepath: Option<&String>) -> Result<Box<dyn Read + Send>> {
    let handle = match_input(filepath)?;
    let (pass, _comp) = niffler::send::get_reader(handle)?;
    Ok(Box::new(pass))
}

pub fn match_output(filepath: Option<&String>) -> Result<Box<dyn Write + Send>> {
    if let Some(filepath) = filepath {
        let handle = File::create(filepath).map(BufWriter::new)?;
        Ok(Box::new(handle))
    } else {
        let handle = BufWriter::new(stdout());
        Ok(Box::new(handle))
    }
}
