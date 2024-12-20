use std::io::{Read, Write};

use anyhow::Result;
use ibu::{Header, Reader, Record};

use crate::{
    cli::ibu::ArgsView,
    io::{match_input, match_output},
};

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

fn decode_record<'a, 'b>(
    record: Record,
    header: Header,
    barcode_buffer: &'a mut Vec<u8>,
    umi_buffer: &'b mut Vec<u8>,
) -> Result<(&'a str, &'b str, u64)> {
    bitnuc::from_2bit(
        record.barcode(),
        header.barcode_len() as usize,
        barcode_buffer,
    )?;
    let barcode_str = std::str::from_utf8(barcode_buffer)?;

    bitnuc::from_2bit(record.umi(), header.umi_len() as usize, umi_buffer)?;
    let umi_str = std::str::from_utf8(umi_buffer)?;

    Ok((barcode_str, umi_str, record.index()))
}

fn dump_decoded_records<W: Write, R: Read>(
    csv_writer: &mut csv::Writer<W>,
    reader: Reader<R>,
    header: Header,
) -> Result<()> {
    let mut barcode_buffer = Vec::new(); // Reusable buffer for barcode nucleotides
    let mut umi_buffer = Vec::new(); // Reusable buffer for UMI nucleotides
    for record in reader {
        let record = record?;
        let decoded = decode_record(record, header, &mut barcode_buffer, &mut umi_buffer)?;
        csv_writer.serialize(decoded)?;

        barcode_buffer.clear();
        umi_buffer.clear();
    }
    csv_writer.flush()?;
    Ok(())
}

pub fn run(args: &ArgsView) -> Result<()> {
    // Handle IO handles
    let input = match_input(args.input.input.as_ref())?;
    let mut output = match_output(args.options.output.as_ref())?;

    // Initialize the reader and header
    let reader = Reader::new(input)?;
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
