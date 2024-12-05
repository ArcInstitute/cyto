use std::{
    fs::File,
    io::{BufReader, Read, Write},
};

use anyhow::Result;
use ibu::{Header, Reader, Record};

use crate::{cli::ibu::ArgsView, io::match_output};

fn build_csv_writer<W: Write>(writer: W) -> csv::Writer<W> {
    csv::WriterBuilder::default()
        .has_headers(false)
        .delimiter(b'\t')
        .from_writer(writer)
}

fn write_header<W: Write>(header: Header, writer: &mut W) -> Result<()> {
    writeln!(writer, "# IBU")?;
    writeln!(writer, "# version: {}", header.version())?;
    writeln!(writer, "# barcode_len: {}", header.barcode_len())?;
    writeln!(writer, "# umi_len: {}", header.umi_len())?;
    writeln!(writer, "# is_sorted: {}", header.sorted())?;
    Ok(())
}

fn dump_encoded_records<W: Write, R: Read>(
    csv_writer: &mut csv::Writer<W>,
    reader: Reader<R>,
) -> Result<()> {
    for record in reader {
        let record = record?;
        csv_writer.serialize(record)?;
    }
    csv_writer.flush()?;
    Ok(())
}

fn decode_record(record: Record, header: Header) -> Result<(String, String, u64)> {
    let barcode = bitnuc::from_2bit(record.barcode(), header.barcode_len() as usize)?;
    let barcode_str = String::from_utf8(barcode)?;
    let umi = bitnuc::from_2bit(record.umi(), header.umi_len() as usize)?;
    let umi_str = String::from_utf8(umi)?;
    Ok((barcode_str, umi_str, record.index()))
}

fn dump_decoded_records<W: Write, R: Read>(
    csv_writer: &mut csv::Writer<W>,
    reader: Reader<R>,
    header: Header,
) -> Result<()> {
    for record in reader {
        let record = record?;
        let decoded = decode_record(record, header)?;
        csv_writer.serialize(decoded)?;
    }
    csv_writer.flush()?;
    Ok(())
}

pub fn run(args: &ArgsView) -> Result<()> {
    // Handle IO handles
    let handle = File::open(&args.input.input).map(BufReader::new)?;
    let mut output = match_output(args.options.output.as_ref())?;

    // Initialize the reader and header
    let reader = Reader::new(handle)?;
    let header = reader.header();

    // Write the header to the output file
    if !args.options.skip_header {
        write_header(header, &mut output)?;
    }

    // If only the header is requested, return early
    if args.options.header {
        return Ok(());
    }

    // Write the records to the output file
    let mut csv_writer = build_csv_writer(output);

    // Write the records to the output file
    if args.options.decode {
        dump_decoded_records(&mut csv_writer, reader, header)
    } else {
        dump_encoded_records(&mut csv_writer, reader)
    }
}
