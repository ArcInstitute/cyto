use anyhow::Result;
use std::{
    fs::File,
    io::{stdin, stdout, BufReader, BufWriter, Read, Write},
};

pub fn match_input(filepath: Option<&String>) -> Result<Box<dyn Read>> {
    if let Some(filepath) = filepath {
        let handle = File::open(filepath).map(BufReader::new)?;
        Ok(Box::new(handle))
    } else {
        let handle = BufReader::new(stdin());
        Ok(Box::new(handle))
    }
}

pub fn match_output(filepath: Option<&String>) -> Result<Box<dyn Write>> {
    if let Some(filepath) = filepath {
        let handle = File::create(filepath).map(BufWriter::new)?;
        Ok(Box::new(handle))
    } else {
        let handle = BufWriter::new(stdout());
        Ok(Box::new(handle))
    }
}
