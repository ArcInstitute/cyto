use anyhow::Result;
use cyto::PairedReader;
use std::{
    fs::File,
    io::{stdout, BufWriter, Write},
};

use crate::cli::ArgsBus;

fn match_output(filepath: Option<String>) -> Result<Box<dyn Write>> {
    if let Some(filepath) = filepath {
        let handle = File::create(filepath).map(BufWriter::new)?;
        Ok(Box::new(handle))
    } else {
        let handle = BufWriter::new(stdout());
        Ok(Box::new(handle))
    }
}

pub fn run(args: ArgsBus) -> Result<()> {
    let mut reader = PairedReader::new(&args.input.r1, &args.input.r2)?;
    let writer = match_output(args.options.output)?;
    reader.write_to(writer, args.geometry.barcode, args.geometry.umi)
}
