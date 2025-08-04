use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Read},
    ops::Add,
    path::Path,
};

use anyhow::{Context, Result, bail};
use bitnuc::as_2bit;
use cyto_cli::ibu::ArgsBarcode;
use cyto_io::{match_input, match_output, match_output_stderr};
use ibu::{Reader, Record};
use log::{debug, info};
use serde::Serialize;

fn open_whitelist<P: AsRef<Path>>(path: P) -> Result<Box<dyn Read + Send>> {
    debug!("Opening whitelist file: {}", path.as_ref().display());
    let file =
        File::open(&path).context(format!("Unable to open path: {}", path.as_ref().display()))?;
    let (passthrough, format) = niffler::send::get_reader(Box::new(file))?;
    match format {
        niffler::send::compression::Format::No => {}
        _ => debug!("Transparent decompression: {format:?}"),
    }
    Ok(passthrough)
}

fn encode_whitelist(
    reader: Box<dyn Read + Send>,
) -> Result<(HashMap<u64, usize>, Vec<u64>, usize)> {
    let bufreader = BufReader::new(reader);
    let mut keys = HashMap::new();
    let mut keys_vec = Vec::new();
    let mut size = 0;
    for line in bufreader.lines() {
        let line = line?;
        if size == 0 {
            size = line.len();
        } else if size != line.len() {
            bail!("All keys in the whitelist must be the same length");
        }
        let ebuf = as_2bit(line.as_bytes())?;
        keys.insert(ebuf, 0);
        keys_vec.push(ebuf);
    }
    Ok((keys, keys_vec, size))
}

#[derive(Debug, Clone, Copy)]
pub enum Correction {
    Corrected(u64),
    Ambiguous,
    Unchanged,
}

#[derive(Debug, Clone)]
pub struct Whitelist {
    /// The set of keys in the whitelist and their abundances
    keys: HashMap<u64, usize>,
    /// A vector of keys in the whitelist (identical to `keys` but in a different format)
    key_vec: Vec<u64>,
    /// The size of each sequence in the whitelist
    slen: usize,
    /// The mismatch table for fast correction
    mismatch_table: bitnuc_mismatch::MismatchTable,
    /// The ambiguous mismatch table for fast identification of ambiguous parents
    ambiguous_table: bitnuc_mismatch::AmbiguousMismatchTable,
}
impl Whitelist {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let reader = open_whitelist(path)?;

        debug!("bitnuc encoding whitelist");
        let (keys, key_vec, slen) = encode_whitelist(reader)?;
        if slen > 32 {
            bail!("The whitelist keys must be 32 nucleotides or less");
        }

        // Generate the mismatch table using the bitnuc-mismatch library
        info!("Building disambiguated mismatch table from whitelist...");
        let (mismatch_table, ambiguous_table) =
            bitnuc_mismatch::build_mismatch_table_with_ambiguous(&key_vec, slen)?;
        info!("Finished disambiguation");

        Ok(Self {
            keys,
            key_vec,
            slen,
            mismatch_table,
            ambiguous_table,
        })
    }

    /// Checks if the whitelist contains the given key
    pub fn contains(&self, key: u64) -> bool {
        self.keys.contains_key(&key)
    }

    /// Increments the abundance of the given key in the whitelist
    pub fn increment(&mut self, key: u64) {
        *self.keys.get_mut(&key).unwrap() += 1;
    }

    /// Corrects the given key to a key in the whitelist if the key is within the given hamming distance.
    ///
    /// Will return `None` if no key in the whitelist is within the given hamming distance
    /// or if the key can be corrected to multiple keys in the whitelist.
    pub fn correct_to(&self, key: u64, distance: u32) -> Correction {
        // If distance is 0, only exact matches are allowed
        if distance == 0 {
            return if self.contains(key) {
                Correction::Unchanged
            } else {
                Correction::Ambiguous
            };
        }

        // If distance is 1, use the precomputed mismatch table
        if distance == 1 {
            // If the key is in the whitelist, return unchanged
            if self.contains(key) {
                return Correction::Unchanged;
            // If the key is in the mismatch table, return the corrected parent
            } else if let Some(&parent) = self.mismatch_table.get(&key) {
                return Correction::Corrected(parent);
            }
            // The key is not in the mismatch table or whitelist
            return Correction::Ambiguous;
        }

        // For distances > 1, fall back to the old method with hdist_scalar
        let mut corrected = None;
        for &k in &self.key_vec {
            if bitnuc::hdist_scalar(k, key, self.slen).expect("Failure in calculating hdist_scalar")
                <= distance
            {
                if corrected.is_some() {
                    return Correction::Ambiguous;
                }
                corrected = Some(k);
            }
        }

        corrected.map_or(Correction::Unchanged, Correction::Corrected)
    }

    pub fn ambiguously_correct_to_(&self, key: u64) -> Correction {
        if let Some(parents) = self.ambiguous_table.get(&key) {
            let parent_counts = parents
                .iter()
                .map(|k| (self.keys.get(k).expect("Error in parent lookup"), k))
                .collect::<HashMap<_, _>>();

            // All parents have the same count (ambiguous)
            if parent_counts.len() == 1 {
                Correction::Ambiguous
            } else {
                parent_counts
                    .iter()
                    .max_by_key(|&(count, _)| count)
                    .map(|(_, k)| Correction::Corrected(**k))
                    .expect("Error in finding max count")
            }
        } else {
            Correction::Ambiguous
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CorrectStats {
    /// The total number of records
    pub total: u64,
    /// The number of records that matched the whitelist
    pub matched: u64,
    /// The number of records that were corrected
    pub corrected: u64,
    /// The number of records that were corrected via counts
    pub corrected_via_counts: u64,
    /// The number of records with ambiguous corrections
    pub ambiguous: u64,
    /// The number of records that were not corrected
    pub unchanged: u64,
}
impl Add for CorrectStats {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            total: self.total + rhs.total,
            matched: self.matched + rhs.matched,
            corrected: self.corrected + rhs.corrected,
            ambiguous: self.ambiguous + rhs.ambiguous,
            unchanged: self.unchanged + rhs.unchanged,
            corrected_via_counts: self.corrected_via_counts + rhs.corrected_via_counts,
        }
    }
}

#[derive(Serialize)]
pub struct FormattedStats {
    total: u64,
    matched: u64,
    corrected: u64,
    corrected_via_counts: u64,
    ambiguous: u64,
    unchanged: u64,
    frac_matched: f64,
    frac_corrected: f64,
    frac_corrected_via_counts: f64,
    frac_ambiguous: f64,
    frac_unchanged: f64,
}
impl FormattedStats {
    pub fn new(stats: CorrectStats) -> Self {
        Self {
            total: stats.total,
            matched: stats.matched,
            corrected: stats.corrected,
            corrected_via_counts: stats.corrected_via_counts,
            ambiguous: stats.ambiguous,
            unchanged: stats.unchanged,
            frac_matched: stats.matched as f64 / stats.total as f64,
            frac_corrected: stats.corrected as f64 / stats.total as f64,
            frac_corrected_via_counts: stats.corrected_via_counts as f64 / stats.total as f64,
            frac_ambiguous: stats.ambiguous as f64 / stats.total as f64,
            frac_unchanged: stats.unchanged as f64 / stats.total as f64,
        }
    }
}

fn write_statistics<P: AsRef<Path>>(path: Option<P>, stats: CorrectStats) -> Result<()> {
    let mut writer = match_output_stderr(path)?;
    let format_stats = FormattedStats::new(stats);
    serde_json::to_writer_pretty(&mut writer, &format_stats)?;
    writer.flush()?;
    Ok(())
}

/// Prebuild whitelist so multiple threads deduplicate work in building mismatch table.
pub fn run_with_prebuilt_whitelist(args: &ArgsBarcode, mut whitelist: Whitelist) -> Result<()> {
    // Build IO handles
    let input = match_input(args.input.input.as_ref())?;

    // Initialize the reader and header
    let reader = Reader::new(input)?;
    let header = reader.header();

    // Write the header to the output file
    let mut output = match_output(args.options.output.as_ref())?;
    header.write_bytes(&mut output)?;

    // Process the records sequentially
    let mut stats = CorrectStats::default();
    let mut second_pass = Vec::new();

    debug!("Starting first pass...");
    for record in reader {
        let record = record?;
        let barcode = record.barcode();
        stats.total += 1;

        // Case where barcode is in the whitelist without error
        match whitelist.correct_to(barcode, args.options.distance) {
            Correction::Ambiguous => {
                if args.options.skip_second_pass {
                    stats.ambiguous += 1;
                    if args.options.include {
                        record.write_bytes(&mut output)?;
                    }
                } else {
                    second_pass.push(record); // Record is ambiguous - will try to resolve in second pass
                }
            }
            Correction::Unchanged => {
                stats.matched += 1;
                stats.unchanged += 1;
                whitelist.increment(barcode);
                record.write_bytes(&mut output)?;
            }
            Correction::Corrected(corrected) => {
                stats.matched += 1;
                stats.corrected += 1;
                whitelist.increment(corrected);
                let new_record = Record::new(corrected, record.umi(), record.index());
                new_record.write_bytes(&mut output)?;
            }
        }
    }

    if !second_pass.is_empty() {
        debug!("Starting second pass (ambiguous subset)...");
        for record in second_pass {
            match whitelist.ambiguously_correct_to_(record.barcode()) {
                Correction::Ambiguous => {
                    stats.ambiguous += 1;
                    // Write ambiguous unless user wants to remove
                    if args.options.include {
                        record.write_bytes(&mut output)?;
                    }
                }
                Correction::Unchanged => {
                    stats.matched += 1;
                    stats.unchanged += 1;
                    record.write_bytes(&mut output)?;
                }
                Correction::Corrected(corrected) => {
                    stats.matched += 1;
                    stats.corrected += 1;
                    stats.corrected_via_counts += 1;
                    let new_record = Record::new(corrected, record.umi(), record.index());
                    new_record.write_bytes(&mut output)?;
                }
            }
        }
    }

    // Flush the output
    output.flush()?;

    // Write the statistics to stderr
    write_statistics(args.options.log.as_ref(), stats)?;
    Ok(())
}

pub fn run(args: &ArgsBarcode) -> Result<()> {
    let whitelist = Whitelist::from_path(&args.options.whitelist)?;
    run_with_prebuilt_whitelist(args, whitelist)
}
