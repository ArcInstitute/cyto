use anyhow::{Result, bail};
use ibu::{Reader, Record};
use petgraph::{Graph, algo::kosaraju_scc, graph::NodeIndex};

use cyto_cli::ibu::ArgsUmi;
use cyto_io::{match_input, match_output};

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

            if x.index() == y.index() && bitnuc::hdist_scalar(x.umi(), y.umi(), umi_len)? <= 1 {
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
    for component in kosaraju_scc(&graph) {
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

pub fn run(args: &ArgsUmi) -> Result<()> {
    // Build IO handles
    let input = match_input(args.input.input.as_ref())?;

    // Initialize the reader and header
    let reader = Reader::new(input)?;
    let header = reader.header();

    // Initialize the output writer
    let mut output = match_output(args.options.output.as_ref())?;
    header.write_bytes(&mut output)?;

    let mut reader_iter = reader.into_iter();
    let mut last_record = if let Some(record) = reader_iter.next() {
        record?
    } else {
        bail!("No records found in input file")
    };

    let mut num_corrections = 0;
    let mut num_records = 0;
    let mut barcode_set = Vec::new();
    let mut corrected_set = Vec::new();
    barcode_set.push(last_record);
    for record in reader_iter {
        let record = record?;
        if record < last_record {
            bail!("Records are not sorted")
        }

        if record.barcode() == last_record.barcode() {
            barcode_set.push(record);
        } else {
            num_corrections += collapse_barcode_set(
                &mut barcode_set,
                &mut corrected_set,
                header.umi_len() as usize,
            )?;
            barcode_set.clear();

            for record in corrected_set.drain(..) {
                record.write_bytes(&mut output)?;
            }

            barcode_set.push(record);
        }

        last_record = record;
        num_records += 1;
    }

    // Process last barcode set
    num_corrections += collapse_barcode_set(
        &mut barcode_set,
        &mut corrected_set,
        header.umi_len() as usize,
    )?;
    for record in corrected_set.drain(..) {
        record.write_bytes(&mut output)?;
    }

    eprintln!("Total records:          {num_records}");
    eprintln!("Total corrections:      {num_corrections}");
    eprintln!(
        "Percentage corrected:   {:.4}%",
        (num_corrections as f64 / f64::from(num_records)) * 100.0
    );

    Ok(())
}
