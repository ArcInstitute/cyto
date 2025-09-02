use std::io::Write;

use anyhow::{Result, bail};

use cyto_cli::ibu::ArgsReads;
use cyto_io::{match_input, match_output};
use ibu::Reader;
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
        bitnuc::decode(&[barcode], n_bases as usize, dbuf)?;
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

pub fn run(args: &ArgsReads) -> Result<()> {
    let input = match_input(args.input.input.as_ref())?;
    let output = match_output(args.options.output.as_ref())?;
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .has_headers(!args.options.no_header)
        .from_writer(output);

    let reader = Reader::new(input)?;
    let header = reader.header();

    let mut last_record = None;
    let mut dbuf = Vec::new();
    let mut n_reads = 0;
    let mut n_umis = 0;
    for record in reader.into_iter() {
        let record = record?;
        if let Some(last_record) = last_record {
            if record < last_record {
                bail!("Expected sorted IBU input")
            }

            if record.barcode() != last_record.barcode() {
                print_record_stats(
                    &mut writer,
                    last_record.barcode(),
                    n_umis,
                    n_reads,
                    args.options.encoded,
                    header.barcode_len(),
                    &mut dbuf,
                )?;
                n_reads = 1;
                n_umis = 1;
            } else {
                if record.umi() != last_record.umi() {
                    n_umis += 1
                }
                n_reads += 1;
            }
        } else {
            n_reads = 1;
            n_umis = 1;
        }

        last_record = Some(record);
    }

    print_record_stats(
        &mut writer,
        last_record.unwrap().barcode(),
        n_umis,
        n_reads,
        args.options.encoded,
        header.barcode_len(),
        &mut dbuf,
    )?;

    Ok(())
}
