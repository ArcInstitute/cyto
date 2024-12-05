use anyhow::Result;
use std::{
    fs::File,
    io::{stdout, BufWriter, Write},
};

pub fn match_output(filepath: Option<String>) -> Result<Box<dyn Write>> {
    if let Some(filepath) = filepath {
        let handle = File::create(filepath).map(BufWriter::new)?;
        Ok(Box::new(handle))
    } else {
        let handle = BufWriter::new(stdout());
        Ok(Box::new(handle))
    }
}
