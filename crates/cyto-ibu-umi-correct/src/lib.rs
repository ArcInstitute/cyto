use std::{
    collections::BTreeMap,
    io::{Read, Write},
    ops::AddAssign,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, unbounded};
use ibu::{Reader, Record, Writer};
use petgraph::{Graph, graph::NodeIndex};

use cyto_cli::ibu::ArgsUmi;
use cyto_io::{match_input, match_output, match_output_stderr};
use serde::Serialize;

mod parallel;
mod utils;

pub use crate::{parallel::BarcodeSetReader, utils::connected_components_vec};

#[derive(Serialize, Clone, Copy)]
struct Statistics {
    total: usize,
    corrected: usize,
    fraction_corrected: f64,
}
impl Statistics {
    pub fn new(total: usize, corrected: usize) -> Self {
        Self {
            total,
            corrected,
            fraction_corrected: corrected as f64 / total as f64,
        }
    }
}
impl AddAssign for Statistics {
    fn add_assign(&mut self, other: Self) {
        self.total += other.total;
        self.corrected += other.corrected;
        self.fraction_corrected = self.corrected as f64 / self.total as f64;
    }
}

/// Sorts the record set by barcode, index, and UMI.
fn resort_record_set(record_set: &mut [Record]) {
    record_set.sort_by(|a, b| {
        a.barcode
            .cmp(&b.barcode)
            .then(a.index.cmp(&b.index))
            .then(a.umi.cmp(&b.umi))
    });
}

/// Collapses the UMIs of an index set of records
///
/// The index set is the set of records with the same barcode and index but with potentially different UMIs.
///
/// The algorithm first deduplicates by UMI, then creates a graph where:
/// - Each node represents a unique UMI in the set
/// - Each edge represents a pair of UMIs with Hamming Distance <= 1
///
/// We then find the connected components of the graph.
/// Each component represents a set of UMIs that can be collapsed into a single UMI.
/// For each component, the UMI with the highest read count is chosen as the representative, then all
/// records in the index set (including duplicates) whose UMI belongs to a non-representative node are updated.
///
/// This function updates the index set in place and returns the number of corrections made.
fn collapse_index_set(index_set: &mut [Record], umi_len: usize) -> Result<usize> {
    // Collect unique UMIs and their read counts.
    // index_set is sorted by umi, so we can do this in a single pass.
    let mut unique_umis: Vec<u64> = Vec::new();
    let mut umi_counts: Vec<usize> = Vec::new();
    for record in index_set.iter() {
        // can directly check last record because `index_set` is sorted by barcode-index-umi
        if unique_umis.last() == Some(&record.umi) {
            *umi_counts.last_mut().unwrap() += 1;
        } else {
            unique_umis.push(record.umi);
            umi_counts.push(1);
        }
    }

    // Early exit condition: if there are fewer than 2 unique UMIs, no correction is needed.
    let n_unique = unique_umis.len();
    if n_unique < 2 {
        return Ok(0);
    }

    // Otherwise, build a graph of all unique UMIs for the barcode-index
    let mut graph = Graph::new_undirected();
    for idx in 0..n_unique {
        graph.add_node(idx);
    }

    // Add edges between UMIs that are close enough to each other.
    let mut n_edges = 0;
    for i in 0..n_unique {
        for j in i + 1..n_unique {
            if bitnuc::twobit::hdist_scalar(unique_umis[i], unique_umis[j], umi_len)? <= 1 {
                graph.add_edge(NodeIndex::new(i), NodeIndex::new(j), ());
                n_edges += 1;
            }
        }
    }

    // No edges, no corrections
    if n_edges == 0 {
        return Ok(0);
    }

    // Build a per-unique-UMI mapping to its representative UMI.
    // The representative is the UMI with the highest read count in the component.
    //
    // Note: this is on the unique UMIs, not the original records.
    let mut corrected_umi: Vec<u64> = unique_umis.clone();
    for component in connected_components_vec(&graph) {
        if component.len() == 1 {
            continue;
        }

        // Find the representative UMI for this component
        let rep_idx = component
            .iter()
            .max_by_key(|node| umi_counts[node.index()])
            .unwrap()
            .index();
        let rep_umi = unique_umis[rep_idx];

        // Update all UMIs in this component to the representative UMI
        for node in &component {
            if node.index() != rep_idx {
                corrected_umi[node.index()] = rep_umi;
            }
        }
    }

    // Apply corrections to all records, including duplicates
    let mut n_corrections = 0;
    for record in index_set.iter_mut() {
        // Find the representative UMI for this barcode-index pair
        let pos = unique_umis.partition_point(|&u| u < record.umi);
        let rep_umi = corrected_umi[pos];

        // If the UMI is not the representative UMI, update it
        if rep_umi != record.umi {
            *record = Record::new(record.barcode, rep_umi, record.index);
            n_corrections += 1;
        }
    }

    Ok(n_corrections)
}

/// Clusters one-hamming distance connected components of records based on their barcode-index identity.
fn collapse_barcode_set(
    barcode_set: &mut [Record],
    corrected_set: &mut Vec<Record>,
    umi_len: usize,
) -> Result<usize> {
    if barcode_set.len() < 2 {
        for record in barcode_set {
            corrected_set.push(*record);
        }
        return Ok(0);
    }

    // Sorts the barcode set by barcode-index-umi
    resort_record_set(barcode_set);

    // Initialize a reusable index set vector
    let mut index_set = Vec::new();

    // Add the first record to the index set
    let mut set_iter = barcode_set.iter();
    let mut last_record = set_iter.next().unwrap();
    index_set.push(*last_record);

    // Iterate over the remaining records
    let mut n_corrections = 0;
    for record in set_iter {
        // If the index is the same, add the record to the index set
        if record.index == last_record.index {
            index_set.push(*record);

        // If the index is different, collapse the index set and clear it
        } else {
            // Collapse the index set (update records in corrected_set)
            n_corrections += collapse_index_set(&mut index_set, umi_len)?;
            index_set.drain(..).for_each(|r| corrected_set.push(r));

            // Begin a new index set with the current record
            index_set.push(*record);
        }

        // Overwrite the last record with the current record
        last_record = record;
    }

    // Process last barcode set (update records in corrected_set)
    n_corrections += collapse_index_set(&mut index_set, umi_len)?;
    index_set.drain(..).for_each(|r| corrected_set.push(r));

    Ok(n_corrections)
}

fn write_statistics<P: AsRef<Path>>(log_path: Option<P>, stats: Statistics) -> Result<()> {
    let mut writer = match_output_stderr(log_path)?;
    serde_json::to_writer_pretty(&mut writer, &stats)?;
    writer.flush()?;
    Ok(())
}

fn process_records_parallel<R, W>(
    reader: ibu::Reader<R>,
    mut writer: ibu::Writer<W>,
    header: ibu::Header,
    threads: usize,
) -> Result<Statistics>
where
    R: Read + Send + 'static,
    W: Write + Send + 'static,
{
    let preader = BarcodeSetReader::new_shared(reader.into_iter());
    let ticket_counter = Arc::new(AtomicUsize::new(0));

    let (tx, rx): (Sender<(usize, Vec<Record>)>, Receiver<(usize, Vec<Record>)>) = unbounded();

    // Spawn writer thread
    let writer_handle = std::thread::spawn(move || -> Result<()> {
        let mut next_expected = 0;
        let mut buffer: BTreeMap<usize, Vec<Record>> = BTreeMap::new();

        for (ticket, records) in rx {
            buffer.insert(ticket, records);

            // Write all sequential records we have
            while let Some(records) = buffer.remove(&next_expected) {
                writer.write_batch(&records)?;
                next_expected += 1;
            }
        }
        writer.finish()?;
        Ok(())
    });

    let mut handles = Vec::new();
    for _tid in 0..threads {
        let treader = preader.clone();
        let ticket_counter = ticket_counter.clone();
        let tx = tx.clone();

        let handle = std::thread::spawn(move || -> Result<Statistics> {
            let mut num_records = 0;
            let mut num_corrections = 0;
            let mut barcode_set = Vec::new();
            let mut corrected_set = Vec::new();

            loop {
                barcode_set.clear();

                let my_ticket = {
                    let mut reader = treader.lock();

                    // Try to read first
                    if !reader.fill_barcode_set(&mut barcode_set)? {
                        break;
                    }

                    // Get ticket while still holding the lock
                    ticket_counter.fetch_add(1, Ordering::SeqCst)
                }; // Lock released here

                num_records += barcode_set.len();
                num_corrections += collapse_barcode_set(
                    &mut barcode_set,
                    &mut corrected_set,
                    header.umi_len as usize,
                )?;

                // sort the correct set by barcode-umi-index
                corrected_set.sort_unstable();

                // Send to writer (non-blocking)
                tx.send((my_ticket, std::mem::take(&mut corrected_set)))
                    .unwrap();
            }

            Ok(Statistics::new(num_records, num_corrections))
        });
        handles.push(handle);
    }

    drop(tx); // Close channel after all workers are done

    let mut statistics = Statistics::new(0, 0);
    for handle in handles {
        statistics += handle.join().unwrap()?;
    }

    writer_handle.join().unwrap()?;

    Ok(statistics)
}

pub fn run(args: &ArgsUmi) -> Result<()> {
    // Build IO handles
    let input = match_input(args.input.input.as_ref())?;

    // Initialize the reader and header
    let reader = Reader::new(input)?;
    let header = reader.header();

    // Initialize the output writer
    let output = match_output(args.options.output.as_ref())?;
    let writer = Writer::new(output, header)?;

    // Process records in parallel
    let stats = process_records_parallel(reader, writer, header, args.options.threads())?;

    // write output statistics
    write_statistics(args.options.log.as_ref(), stats)?;

    Ok(())
}

#[cfg(test)]
mod testing {
    use ibu::Record;

    use super::*;

    fn rec(umi: u64) -> Record {
        Record::new(0, umi, 0)
    }

    fn sorted(mut records: Vec<Record>) -> Vec<Record> {
        records.sort_by_key(|r| r.umi);
        records
    }

    /// All records share the same UMI — nothing to correct.
    #[test]
    fn test_all_same_umi() {
        let mut index_set = sorted(vec![rec(0), rec(0), rec(0)]);
        let n = collapse_index_set(&mut index_set, 1).unwrap();
        assert_eq!(n, 0);
        assert!(index_set.iter().all(|r| r.umi == 0));
        assert_eq!(index_set.len(), 3);
    }

    /// Two unique UMIs at HD=1, no duplicates — one correction.
    #[test]
    fn test_two_umis_hd1() {
        // umi_len=1: 0b00 vs 0b01 differ in one nucleotide (HD=1)
        let mut index_set = sorted(vec![rec(0b00), rec(0b01)]);
        let n = collapse_index_set(&mut index_set, 1).unwrap();
        assert_eq!(n, 1);
        let rep = index_set[0].umi;
        assert!(index_set.iter().all(|r| r.umi == rep));
    }

    /// The UMI with more supporting reads wins, not the first node in the component.
    #[test]
    fn test_representative_is_highest_count() {
        let umi_a = 0b00u64;
        let umi_b = 0b01u64; // HD=1 from umi_a
        // umi_b has more reads — it should be chosen as the representative
        let mut index_set = sorted(vec![rec(umi_a), rec(umi_b), rec(umi_b), rec(umi_b)]);
        let n = collapse_index_set(&mut index_set, 1).unwrap();
        assert_eq!(n, 1); // only the one umi_a record was corrected
        assert!(index_set.iter().all(|r| r.umi == umi_b));
    }

    /// Two unique UMIs at HD=2 — no correction.
    #[test]
    fn test_two_umis_hd2() {
        // umi_len=2: 0b0000 vs 0b0101 differ in both positions (HD=2)
        let umi_a = 0b0000u64;
        let umi_b = 0b0101u64;
        let mut index_set = sorted(vec![rec(umi_a), rec(umi_b)]);
        let n = collapse_index_set(&mut index_set, 2).unwrap();
        assert_eq!(n, 0);
    }

    /// Many duplicate records — corrections are applied to every copy, not just unique UMIs.
    #[test]
    fn test_many_duplicates_all_corrected() {
        let umi_a = 0b00u64;
        let umi_b = 0b01u64; // HD=1 from umi_a with umi_len=1
        let n_a = 100usize;
        let n_b = 50usize;

        let mut index_set: Vec<Record> = (0..n_a)
            .map(|_| rec(umi_a))
            .chain((0..n_b).map(|_| rec(umi_b)))
            .collect();
        index_set = sorted(index_set);

        let n = collapse_index_set(&mut index_set, 1).unwrap();

        // All n_b records holding umi_b must have been corrected
        assert_eq!(n, n_b);
        assert!(index_set.iter().all(|r| r.umi == umi_a));
    }

    /// Three UMIs forming a chain A-B-C where HD(A,B)=1, HD(B,C)=1, HD(A,C)=2.
    /// All should collapse into a single component via transitivity.
    #[test]
    fn test_chain_transitivity() {
        // umi_len=2: A=0b0000, B=0b0001 (HD 1 from A), C=0b0101 (HD(B,C)=1, HD(A,C)=2)
        let umi_a = 0b0000u64;
        let umi_b = 0b0001u64; // HD(A,B)=1
        let umi_c = 0b0101u64; // HD(B,C)=1, HD(A,C)=2
        let mut index_set = sorted(vec![rec(umi_a), rec(umi_b), rec(umi_c)]);
        let n = collapse_index_set(&mut index_set, 2).unwrap();
        assert_eq!(n, 2);
        let rep = index_set[0].umi;
        assert!(index_set.iter().all(|r| r.umi == rep));
    }
}
