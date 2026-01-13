use std::{
    io::{BufRead, BufReader, Read, Write},
    path::Path,
};

use anyhow::{Result, bail};

use bitnuc::as_2bit;
use cyto_cli::ibu::ArgsReads;
use cyto_io::{match_input, match_output_transparent};
use hashbrown::HashSet;
use ibu::{Header, Reader};
use log::warn;
use serde::Serialize;

#[derive(Serialize)]
pub struct Stats<T> {
    pub barcode: T,
    pub n_umis: usize,
    pub n_reads: usize,
}
impl<T: Serialize> Stats<T> {
    pub fn new(barcode: T, n_umis: usize, n_reads: usize) -> Self {
        Stats {
            barcode,
            n_umis,
            n_reads,
        }
    }
}
impl<'a> Stats<&'a str> {
    pub fn decode(
        barcode: u64,
        n_bases: u32,
        dbuf: &'a mut Vec<u8>,
        n_umis: usize,
        n_reads: usize,
    ) -> Result<Self> {
        dbuf.clear();
        bitnuc::twobit::decode(&[barcode], n_bases as usize, dbuf)?;
        let barcode_str = std::str::from_utf8(dbuf)?;
        Ok(Stats {
            barcode: barcode_str,
            n_umis,
            n_reads,
        })
    }
}

fn print_record_stats<W: Write>(
    output: &mut csv::Writer<W>,
    barcode: u64,
    n_umis: usize,
    n_reads: usize,
    encoded: bool,
    n_bases: u32,
    dbuf: &mut Vec<u8>,
) -> Result<()> {
    if encoded {
        let stats = Stats::new(barcode, n_umis, n_reads);
        output.serialize(stats)?;
    } else {
        let stats = Stats::decode(barcode, n_bases, dbuf, n_umis, n_reads)?;
        output.serialize(stats)?;
    }
    Ok(())
}

fn process_records<R: Read, W: Write>(
    reader: &mut Reader<R>,
    header: &Header,
    writer: &mut csv::Writer<W>,
    whitelist: &Whitelist,
    encoded: bool,
) -> Result<()> {
    let mut last_record = None;
    let mut dbuf = Vec::new();
    let mut n_reads = 0;
    let mut n_umis = 0;
    for record in &mut *reader {
        let record = record?;

        if !whitelist.matches(record.barcode) {
            continue;
        }

        if let Some(last_record) = last_record {
            if record < last_record {
                bail!("Expected sorted IBU input")
            }

            if record.barcode == last_record.barcode {
                if record.umi != last_record.umi {
                    n_umis += 1;
                }
                n_reads += 1;
            } else {
                print_record_stats(
                    writer,
                    last_record.barcode,
                    n_umis,
                    n_reads,
                    encoded,
                    header.bc_len,
                    &mut dbuf,
                )?;
                n_reads = 1;
                n_umis = 1;
            }
        } else {
            n_reads = 1;
            n_umis = 1;
        }

        last_record = Some(record);
    }

    // Print the stats for the last record (if any)
    if let Some(last_record) = last_record {
        print_record_stats(
            writer,
            last_record.barcode,
            n_umis,
            n_reads,
            encoded,
            header.bc_len,
            &mut dbuf,
        )?;
    } else {
        warn!("No records matching whitelist found!");
    }

    Ok(())
}

pub struct Whitelist {
    whitelist: Option<HashSet<u64>>,
}

impl Whitelist {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let reader = match_input(Some(path))?;
        let mut keys = HashSet::new();
        let bufreader = BufReader::new(reader);
        let mut size = 0;
        for line in bufreader.lines() {
            let line = line?;
            if size == 0 {
                size = line.len();
            } else if size != line.len() {
                bail!("All keys in the whitelist must be the same length");
            }
            let ebuf = as_2bit(line.as_bytes())?;
            keys.insert(ebuf);
        }
        let whitelist = Whitelist {
            whitelist: Some(keys),
        };
        Ok(whitelist)
    }

    pub fn from_optional_path<P: AsRef<Path>>(path: Option<P>) -> Result<Self> {
        match path {
            Some(path) => Self::from_path(path),
            None => Ok(Whitelist { whitelist: None }),
        }
    }

    pub fn matches(&self, key: u64) -> bool {
        if let Some(whitelist) = &self.whitelist {
            whitelist.contains(&key)
        } else {
            true
        }
    }
}

pub fn run(args: &ArgsReads) -> Result<()> {
    let input = match_input(args.input.input.as_ref())?;
    let output = match_output_transparent(args.options.output.as_ref())?;
    let whitelist = Whitelist::from_optional_path(args.options.whitelist.as_ref())?;
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .has_headers(!args.options.no_header)
        .from_writer(output);

    let mut reader = Reader::new(input)?;
    let header = reader.header();

    process_records(
        &mut reader,
        &header,
        &mut writer,
        &whitelist,
        args.options.encoded,
    )?;
    writer.flush()?;

    Ok(())
}
