use anyhow::Result;
use ibu::{Header, Reader};
use std::io::Write;

use crate::{
    cli::ibu::ArgsCount,
    io::{match_input, match_output},
};
use cyto::{deduplicate_umis, BarcodeIndexCount};

fn dump_encoded_records<W: Write>(
    csv_writer: &mut csv::Writer<W>,
    records: impl Iterator<Item = BarcodeIndexCount>,
) -> Result<()> {
    for record in records {
        csv_writer.serialize(record)?;
    }
    csv_writer.flush()?;
    Ok(())
}

fn decode_record(record: BarcodeIndexCount, header: Header) -> Result<(String, u64, u64)> {
    let barcode = bitnuc::from_2bit(record.barcode(), header.barcode_len() as usize)?;
    let barcode_str = String::from_utf8(barcode)?;
    Ok((barcode_str, record.count(), record.index()))
}

fn dump_decoded_records<W: Write>(
    csv_writer: &mut csv::Writer<W>,
    records: impl Iterator<Item = BarcodeIndexCount>,
    header: Header,
) -> Result<()> {
    for record in records {
        let decoded = decode_record(record, header)?;
        csv_writer.serialize(decoded)?;
    }
    csv_writer.flush()?;
    Ok(())
}

pub fn run(args: &ArgsCount) -> Result<()> {
    let input = match_input(args.input.input.as_ref())?;

    let reader = Reader::new(input)?;
    let header = reader.header();
    let counts = deduplicate_umis(reader)?;
    let output_handle = match_output(args.output.as_ref())?;

    // Write output
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .from_writer(output_handle);

    if args.compressed {
        dump_encoded_records(&mut writer, counts.iter_counts())
    } else {
        dump_decoded_records(&mut writer, counts.iter_counts(), header)
    }
}
