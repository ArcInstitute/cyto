use std::io::Write;

use anyhow::Result;
use cyto_cli::ibu::ArgsCount;
use cyto_core::{BarcodeIndexCount, deduplicate_umis};
use cyto_io::{match_input, match_output};
use ibu::{Header, Reader};

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

#[allow(clippy::cast_possible_truncation)]
fn dump_encoded_records_features<W: Write>(
    csv_writer: &mut csv::Writer<W>,
    records: impl Iterator<Item = BarcodeIndexCount>,
    features: &[String],
) -> Result<()> {
    for record in records {
        let tuple = (
            record.barcode(),
            &features[record.index() as usize],
            record.count(),
        );
        csv_writer.serialize(tuple)?;
    }
    csv_writer.flush()?;
    Ok(())
}

fn decode_record(
    record: BarcodeIndexCount,
    header: Header,
    barcode_buffer: &mut Vec<u8>,
) -> Result<(&str, u64, u64)> {
    bitnuc::from_2bit(
        record.barcode(),
        header.barcode_len() as usize,
        barcode_buffer,
    )?;
    let barcode_str = std::str::from_utf8(barcode_buffer)?;
    Ok((barcode_str, record.count(), record.index()))
}

fn dump_decoded_records<W: Write>(
    csv_writer: &mut csv::Writer<W>,
    records: impl Iterator<Item = BarcodeIndexCount>,
    header: Header,
) -> Result<()> {
    let mut barcode_buffer = Vec::new(); // Reusable buffer for barcode nucleotides
    for record in records {
        let decoded = decode_record(record, header, &mut barcode_buffer)?;
        csv_writer.serialize(decoded)?;
        barcode_buffer.clear();
    }
    csv_writer.flush()?;
    Ok(())
}

#[allow(clippy::cast_possible_truncation)]
fn dump_decoded_records_features<W: Write>(
    csv_writer: &mut csv::Writer<W>,
    records: impl Iterator<Item = BarcodeIndexCount>,
    header: Header,
    features: &[String],
) -> Result<()> {
    let mut barcode_buffer = Vec::new(); // Reusable buffer for barcode nucleotides
    for record in records {
        bitnuc::from_2bit(
            record.barcode(),
            header.barcode_len() as usize,
            &mut barcode_buffer,
        )?;
        let barcode_str = std::str::from_utf8(&barcode_buffer)?;
        let decoded = (
            barcode_str,
            &features[record.index() as usize],
            record.count(),
        );
        csv_writer.serialize(decoded)?;
        barcode_buffer.clear();
    }
    csv_writer.flush()?;
    Ok(())
}

fn load_features(path: Option<&String>) -> Result<Option<Vec<String>>> {
    if let Some(path) = path {
        let features = std::fs::read_to_string(path)?;
        Ok(Some(features.lines().map(String::from).collect()))
    } else {
        Ok(None)
    }
}

pub fn run(args: &ArgsCount) -> Result<()> {
    let input = match_input(args.input.input.as_ref())?;
    let features = load_features(args.features.as_ref())?;
    let max_index = if let Some(features) = &features {
        features.len()
    } else {
        usize::MAX
    };

    let reader = Reader::new(input)?;
    let header = reader.header();
    let counts = deduplicate_umis(reader, max_index as u64)?;
    let output_handle = match_output(args.output.as_ref())?;

    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .from_writer(output_handle);

    match (features, args.compressed) {
        (Some(features), true) => {
            dump_encoded_records_features(&mut writer, counts.iter_counts(), &features)
        }
        (Some(features), false) => {
            dump_decoded_records_features(&mut writer, counts.iter_counts(), header, &features)
        }
        (None, true) => dump_encoded_records(&mut writer, counts.iter_counts()),
        (None, false) => dump_decoded_records(&mut writer, counts.iter_counts(), header),
    }
}
