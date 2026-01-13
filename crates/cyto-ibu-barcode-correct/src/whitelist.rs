use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
    sync::Arc,
};

use anyhow::{Context, Result, bail};
use bitnuc::as_2bit;
use hashbrown::HashMap;
use log::{debug, info};

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
    /// The mismatch table for fast correction
    mismatch_table: Arc<bitnuc_mismatch::MismatchTable>,
    /// The ambiguous mismatch table for fast identification of ambiguous parents
    ambiguous_table: Arc<bitnuc_mismatch::AmbiguousMismatchTable>,
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

        // Wrap the mismatch tables in Arcs for shared ownership
        let mismatch_table = Arc::new(mismatch_table);
        let ambiguous_table = Arc::new(ambiguous_table);

        Ok(Self {
            keys,
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
    pub fn correct_to(&self, key: u64, exact: bool) -> Correction {
        // If distance is 0, only exact matches are allowed
        if exact {
            if self.contains(key) {
                Correction::Unchanged
            } else {
                Correction::Ambiguous
            }
        }
        // If distance is 1, use the precomputed mismatch table
        else {
            // If the key is in the whitelist, return unchanged
            if self.contains(key) {
                Correction::Unchanged
            // If the key is in the mismatch table, return the corrected parent
            } else if let Some(&parent) = self.mismatch_table.get(&key) {
                Correction::Corrected(parent)
            } else {
                // The key is not in the mismatch table or whitelist
                Correction::Ambiguous
            }
        }
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
