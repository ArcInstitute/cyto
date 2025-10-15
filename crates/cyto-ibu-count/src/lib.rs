use std::{io::Write, path::Path};

use anyhow::{Result, bail};
use cyto_cli::ibu::ArgsCount;
use cyto_core::{BarcodeIndexCount, BarcodeIndexCounts, deduplicate_umis};
use cyto_io::{match_input, match_output};
use gzp::{
    ZWriter,
    deflate::Gzip,
    par::compress::{ParCompress, ParCompressBuilder},
};
use hashbrown::HashMap;
use ibu::{Header, Reader};
use log::{debug, error, info};

/// Extends a barcode buffer with an optional suffix
fn extend_suffix(buffer: &mut Vec<u8>, suffix: Option<&str>) {
    if let Some(suffix) = suffix {
        buffer.push(b'-');
        buffer.extend_from_slice(suffix.as_bytes());
    }
}

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
    suffix: Option<&str>,
) -> Result<()> {
    let mut barcode_buffer = Vec::new(); // Reusable buffer for barcode nucleotides
    for record in records {
        bitnuc::from_2bit(
            record.barcode(),
            header.barcode_len() as usize,
            &mut barcode_buffer,
        )?;

        // handle suffix
        extend_suffix(&mut barcode_buffer, suffix);

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

fn load_features(path: Option<&String>, feature_col: usize) -> Result<Option<Vec<String>>> {
    if let Some(path) = path {
        let features = std::fs::read_to_string(path)?;
        Ok(Some(
            features
                .lines()
                .map(|s| {
                    s.split_whitespace()
                        .nth(feature_col)
                        .expect("empty feature file or missing feature column: {feature_col}")
                })
                .map(String::from)
                .collect(),
        ))
    } else {
        Ok(None)
    }
}

fn aggregate_unit(
    counts: &BarcodeIndexCounts,
    features: &[String],
) -> (BarcodeIndexCounts, Vec<String>) {
    // Creates a LUT to map feature names to unique indices
    let mut aggr_to_uidx = HashMap::new();
    for f in features {
        if !aggr_to_uidx.contains_key(f) {
            aggr_to_uidx.insert(f.to_string(), aggr_to_uidx.len());
        }
    }

    // Creates a vector to store the aggregated feature names
    let mut agg_features = vec![String::new(); aggr_to_uidx.len()];
    for (feature, idx) in &aggr_to_uidx {
        agg_features[*idx] = feature.to_string();
    }

    let mut agg_counts = BarcodeIndexCounts::with_capacity(counts.get_num_barcodes());
    for record in counts.iter_counts() {
        let unit_idx = record.index() as usize;
        let aggr_name = &features[unit_idx];
        let aggr_idx = aggr_to_uidx[aggr_name];
        agg_counts.insert_count(record.barcode(), aggr_idx as u64, record.count());
    }

    (agg_counts, agg_features)
}

fn write_counts_tsv<P: AsRef<Path>>(
    path: Option<&P>,
    counts: &BarcodeIndexCounts,
    features: Option<Vec<String>>,
    header: Header,
    twobit_compressed: bool,
    suffix: Option<&str>,
) -> Result<()> {
    if let Some(path) = path
        && path.as_ref().exists()
        && path.as_ref().is_dir()
    {
        error!(
            "Output path already exists and is a directory. Only `--mtx` can accept a directory.",
        );
        bail!(
            "Output path already exists and is a directory:\n{}",
            path.as_ref().display()
        )
    }

    let output_handle = match_output(path.as_ref())?;
    let path_name = if let Some(ref path) = path {
        path.as_ref().to_str().expect("Path should be valid UTF-8")
    } else {
        "stdout"
    };

    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .from_writer(output_handle);

    match (features, twobit_compressed) {
        (Some(features), true) => {
            dump_encoded_records_features(&mut writer, counts.iter_counts(), &features)
        }
        (Some(features), false) => dump_decoded_records_features(
            &mut writer,
            counts.iter_counts(),
            header,
            &features,
            suffix,
        ),
        (None, true) => dump_encoded_records(&mut writer, counts.iter_counts()),
        (None, false) => dump_decoded_records(&mut writer, counts.iter_counts(), header),
    }?;
    info!("Finished writing TSV counts to {path_name}");

    Ok(())
}

fn write_counts_mtx<P: AsRef<Path>>(
    outdir: P,
    counts: &BarcodeIndexCounts,
    features: &[String],
    header: Header,
    zthreads: usize,
    suffix: Option<&str>,
) -> Result<()> {
    // make the output directory
    debug!("Creating output directory: {}", outdir.as_ref().display());
    std::fs::create_dir_all(outdir.as_ref())?;

    let mtx_path = outdir.as_ref().join("matrix.mtx.gz");
    let barcodes_path = outdir.as_ref().join("barcodes.tsv.gz");
    let features_path = outdir.as_ref().join("features.tsv.gz");

    let mtx_handle = match_output(Some(mtx_path))?;
    let barcodes_handle = match_output(Some(barcodes_path))?;
    let features_handle = match_output(Some(features_path))?;

    let mut mtx_handle: ParCompress<Gzip, _> = ParCompressBuilder::new()
        .num_threads(zthreads)?
        .from_writer(mtx_handle);

    let mut barcodes_handle: ParCompress<Gzip, _> = ParCompressBuilder::new()
        .num_threads(zthreads)?
        .from_writer(barcodes_handle);

    let mut features_handle: ParCompress<Gzip, _> = ParCompressBuilder::new()
        .num_threads(zthreads)?
        .from_writer(features_handle);

    // write the features file
    for feature in features {
        writeln!(features_handle, "{feature}")?;
    }
    features_handle.finish()?;

    mtx_handle.write_all(b"%%MatrixMarket matrix coordinate real general\n")?;
    mtx_handle.write_all(b"% Generated by cyto-ibu-count\n")?;
    writeln!(
        mtx_handle,
        "{} {} {}",
        features.len(),            // number of features
        counts.get_num_barcodes(), // number of barcodes
        counts.get_nnz()           // number of non-zero elements
    )?;

    let mut mtx_writer = csv::WriterBuilder::new()
        .delimiter(b' ')
        .from_writer(mtx_handle);

    // barcode to index
    let mut bc_idx_map = HashMap::new();
    let mut dbuf = Vec::default();
    for record in counts.iter_counts() {
        let bc_idx = if bc_idx_map.contains_key(&record.barcode()) {
            // barcode exists already
            *bc_idx_map.get(&record.barcode()).unwrap()
        } else {
            // decode the barcode
            dbuf.clear();
            bitnuc::from_2bit(record.barcode(), header.barcode_len() as usize, &mut dbuf)?;

            // handle suffix
            extend_suffix(&mut dbuf, suffix);

            // write barcode to file
            barcodes_handle.write_all(&dbuf)?;
            barcodes_handle.write_all(b"\n")?;

            // insert new barcode
            let bc_idx = bc_idx_map.len();
            bc_idx_map.insert(record.barcode(), bc_idx);
            bc_idx
        };

        let mtx_record = (
            record.index() + 1, // transcript / gene index
            bc_idx + 1,         // barcode index
            record.count(),     // count
        );
        mtx_writer.serialize(mtx_record)?;
    }
    mtx_writer.flush()?;
    barcodes_handle.finish()?;

    info!(
        "Finished mtx creation in directory: {}",
        outdir.as_ref().display()
    );

    Ok(())
}

pub fn run(args: &ArgsCount) -> Result<()> {
    let input = match_input(args.input.input.as_ref())?;
    let mut features = load_features(args.features.as_ref(), args.feature_col)?;
    let max_index = if let Some(features) = &features {
        features.len()
    } else {
        usize::MAX
    };

    let reader = Reader::new(input)?;
    let header = reader.header();
    let mut counts = deduplicate_umis(reader, max_index as u64)?;

    // aggregate the units if features are present
    if let Some(tx_features) = &features {
        // skip if feature col is the `unit` column
        if args.feature_col != 0 {
            let (agg_counts, agg_features) = aggregate_unit(&counts, tx_features);
            counts = agg_counts;
            features = Some(agg_features);
        }
    }

    if args.mtx {
        write_counts_mtx(
            args.output
                .as_ref()
                .expect("Must provide an output directory to write MTX"),
            &counts,
            features
                .expect("Must provide a feature file to write MTX")
                .as_slice(),
            header,
            args.num_threads,
            args.suffix.as_deref(),
        )
    } else {
        write_counts_tsv(
            args.output.as_ref(),
            &counts,
            features,
            header,
            args.compressed,
            args.suffix.as_deref(),
        )
    }
}
