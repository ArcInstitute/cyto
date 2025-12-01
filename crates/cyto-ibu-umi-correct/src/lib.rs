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
use ibu::{Reader, Record};
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
        a.barcode()
            .cmp(&b.barcode())
            .then(a.index().cmp(&b.index()))
            .then(a.umi().cmp(&b.umi()))
    });
}

/// Collapses the UMIs of an index set of records
///
/// The index set is the set of records with the same barcode and index but with potentially different UMIs.
///
/// The algorithm first creates a graph where:
/// - Each node represents a record in the set
/// - Each edge represents a pair of records with a UMI Hamming Distance <= 1
///
/// We then use Kosaraju's algorithm to find the strongly connected components of the graph.
/// Each component represents a set of UMIs that can be collapsed into a single UMI.
/// For each component, we take the first UMI (arbitrary) as the representative UMI, then update all UMIs in the component to the representative UMI.
///
/// This function updates the index set in place and returns the number of corrections made.
fn collapse_index_set(index_set: &mut [Record], umi_len: usize) -> Result<usize> {
    let n_records = index_set.len();
    let mut graph = Graph::new_undirected();
    for idx in 0..n_records {
        graph.add_node(idx);
    }

    let mut n_edges = 0;
    for i in 0..n_records {
        for j in i + 1..n_records {
            let x = index_set[i];
            let y = index_set[j];

            if x.index() == y.index()
                && bitnuc::twobit::hdist_scalar(x.umi(), y.umi(), umi_len)? <= 1
            {
                graph.add_edge(NodeIndex::new(i), NodeIndex::new(j), ());
                n_edges += 1;
            }
        }
    }

    // No edges, no corrections
    if n_edges == 0 {
        return Ok(0);
    }

    let mut n_corrections = 0;
    for component in connected_components_vec(&graph) {
        // Skip single-node components (no need to collapse)
        if component.len() == 1 {
            continue;
        }

        // Select a representative UMI
        let parent = index_set[component[0].index()];
        for child_idx in &component[1..] {
            // Identify the child record
            let child = index_set[child_idx.index()];

            // Skip if child is a duplicate of parent
            if child != parent {
                // Update the child's UMI to match the parent's (in-place)
                index_set[child_idx.index()] =
                    Record::new(child.barcode(), parent.umi(), child.index());

                n_corrections += 1;
            }
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
        if record.index() == last_record.index() {
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
    output: W,
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
        let mut output = output;
        let mut next_expected = 0;
        let mut buffer: BTreeMap<usize, Vec<Record>> = BTreeMap::new();

        for (ticket, records) in rx {
            buffer.insert(ticket, records);

            // Write all sequential records we have
            while let Some(records) = buffer.remove(&next_expected) {
                for record in records {
                    record.write_bytes(&mut output)?;
                }
                output.flush()?;
                next_expected += 1;
            }
        }
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
                    header.umi_len() as usize,
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
    let mut output = match_output(args.options.output.as_ref())?;
    header.write_bytes(&mut output)?;

    // Process records in parallel
    let stats = process_records_parallel(reader, output, header, args.options.threads())?;

    // write output statistics
    write_statistics(args.options.log.as_ref(), stats)?;

    Ok(())
}
