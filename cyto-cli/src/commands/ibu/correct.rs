use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader, Read},
    ops::Add,
    sync::Arc,
};

use anyhow::{bail, Result};
use bitnuc::as_2bit;
use ibu::{Reader, Record};
use parking_lot::Mutex;
use rayon::prelude::*;

use crate::{
    cli::ibu::ArgsCorrect,
    io::{match_input, match_output},
};

fn open_whitelist(path: &str) -> Result<Box<dyn Read + Send>> {
    let file = File::open(path)?;
    let (passthrough, _format) = niffler::send::get_reader(Box::new(file))?;
    Ok(passthrough)
}

fn encode_whitelist(reader: Box<dyn Read + Send>) -> Result<(HashSet<u64>, Vec<u64>, usize)> {
    let bufreader = BufReader::new(reader);
    let mut keys = HashSet::new();
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
        keys.insert(ebuf);
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
    /// The set of keys in the whitelist
    keys: HashSet<u64>,
    /// A vector of keys in the whitelist (identical to `keys` but in a different format)
    key_vec: Vec<u64>,
    /// The size of each sequence in the whitelist
    slen: usize,
    /// The mismatch table for fast correction
    mismatch_table: bitnuc_mismatch::MismatchTable,
}
impl Whitelist {
    pub fn from_path(path: &str) -> Result<Self> {
        let reader = open_whitelist(path)?;
        let (keys, key_vec, slen) = encode_whitelist(reader)?;
        if slen > 32 {
            bail!("The whitelist keys must be 32 nucleotides or less");
        }

        // Generate the mismatch table using the bitnuc-mismatch library
        let mismatch_table = bitnuc_mismatch::build_mismatch_table(&key_vec, slen)?;

        Ok(Self {
            keys,
            key_vec,
            slen,
            mismatch_table,
        })
    }

    /// Checks if the whitelist contains the given key
    pub fn contains(&self, key: u64) -> bool {
        self.keys.contains(&key)
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
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CorrectStats {
    /// The total number of records
    pub total: u64,
    /// The number of records that matched the whitelist
    pub matched: u64,
    /// The number of records that were corrected
    pub corrected: u64,
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
        }
    }
}

fn set_threads(num_threads: usize) -> Result<()> {
    let threads = match num_threads {
        0 => num_cpus::get(),
        x => num_cpus::get().min(x),
    };
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()?;
    Ok(())
}

fn write_statistics(stats: CorrectStats, remove: bool) {
    let total = stats.total;
    let matched = stats.matched;
    let corrected = stats.corrected;
    let ambiguous = stats.ambiguous;
    let unchanged = stats.unchanged;
    let written = if remove {
        total - ambiguous - unchanged
    } else {
        total
    };

    let frac_matched = matched as f64 / total as f64;
    let frac_corrected = corrected as f64 / total as f64;
    let frac_ambiguous = ambiguous as f64 / total as f64;
    let frac_unchanged = unchanged as f64 / total as f64;
    let frac_written = written as f64 / total as f64;

    eprintln!("Total records: {total}");
    eprintln!(
        "Matched records: {} ({:.2}%)",
        matched,
        frac_matched * 100.0
    );
    eprintln!(
        "Corrected records: {} ({:.2}%)",
        corrected,
        frac_corrected * 100.0
    );
    eprintln!(
        "Ambiguous records: {} ({:.2}%)",
        ambiguous,
        frac_ambiguous * 100.0
    );
    eprintln!(
        "Unchanged records: {} ({:.2}%)",
        unchanged,
        frac_unchanged * 100.0
    );
    eprintln!(
        "Written records: {} ({:.2}%)",
        written,
        frac_written * 100.0
    );
}

pub fn run(args: &ArgsCorrect) -> Result<()> {
    // Build IO handles
    let input = match_input(args.input.input.as_ref())?;
    let whitelist = Whitelist::from_path(&args.options.whitelist)?;

    // Set the number of threads for parallel processing
    set_threads(args.options.num_threads)?;

    // Initialize the reader and header
    let reader = Reader::new(input)?;
    let header = reader.header();

    // Write the header to the output file
    let mut output = match_output(args.options.output.as_ref())?;
    header.write_bytes(&mut output)?;

    // Process the records in parallel
    let stats = Mutex::new(CorrectStats::default());
    let output = Arc::new(Mutex::new(output));
    reader
        .into_iter()
        .par_bridge()
        .filter_map(|record| -> Option<Result<Record>> {
            let record = match record {
                Ok(record) => record,
                Err(why) => {
                    return Some(Err(anyhow::Error::new(why)));
                }
            };
            let barcode = record.barcode();

            stats.lock().total += 1;

            if whitelist.contains(barcode) {
                stats.lock().matched += 1;
                Some(Ok(record))
            } else {
                match whitelist.correct_to(barcode, args.options.distance) {
                    Correction::Ambiguous => {
                        stats.lock().ambiguous += 1;
                        if args.options.remove {
                            None
                        } else {
                            Some(Ok(record))
                        }
                    }
                    Correction::Unchanged => {
                        stats.lock().unchanged += 1;
                        if args.options.remove {
                            None
                        } else {
                            Some(Ok(record))
                        }
                    }
                    Correction::Corrected(bc) => {
                        stats.lock().corrected += 1;
                        Some(Ok(Record::new(bc, record.umi(), record.index())))
                    }
                }
            }
        })
        .try_for_each(|record| -> Result<()> {
            let record = record?;
            {
                let mut lock = output.lock();
                record.write_bytes(&mut lock.as_mut())?;
            }
            Ok(())
        })?;

    // Flush the output
    output.lock().flush()?;

    // Write the statistics to stderr
    let stats = stats.into_inner();
    write_statistics(stats, args.options.remove);
    Ok(())
}
