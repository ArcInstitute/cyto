# cyto-ibu-umi-correct

## Purpose

Corrects UMI sequencing errors using graph-based connected components. Groups records by barcode, builds a graph where UMIs within hamming distance 1 are connected, finds connected components, and collapses each component to a representative UMI. Expects sorted IBU input.

## Key Source Files

- `src/lib.rs` — Core correction logic:
  - `collapse_index_set()` — Builds undirected graph of UMIs within HD<=1 (using `bitnuc::twobit::hdist_scalar`), finds connected components via `petgraph`, selects first UMI as representative, updates records in-place
  - `collapse_barcode_set()` — Groups records by barcode-index pair, calls `collapse_index_set()` on each group
  - `process_records_parallel()` — Multi-threaded processing: worker threads pull barcode sets from shared reader, process them, and send corrected records to a writer thread via ticket-based ordering (ensures deterministic output)
  - `run()` — Entry point
- `src/parallel.rs` — `BarcodeSetReader`: shared iterator wrapper that fills a `Vec<Record>` with all records sharing the same barcode. Validates sort order. Used with `Arc<Mutex<...>>` for thread-safe access.
- `src/utils.rs` — `connected_components_vec()`: UnionFind-based connected components algorithm. Returns `Vec<Vec<NodeId>>`. Extensive unit tests including random graph validation against petgraph's Kosaraju/Tarjan implementations.

## Key Types

- `BarcodeSetReader<It>` — Thread-safe barcode-grouped record reader
- `Statistics` — Tracks total records, corrected count, fraction corrected (serialized to JSON)

## Design Patterns

- **Ticket-based ordering**: Each worker thread gets a monotonically increasing ticket number while holding the reader lock. The writer thread uses a `BTreeMap` buffer to reorder results and write them sequentially. This ensures deterministic output regardless of thread scheduling.
- **Barcode-level parallelism**: Each thread processes all records for a complete barcode before moving on, avoiding cross-barcode coordination.

## Dependencies (within workspace)

- `cyto-cli` — `ArgsUmi` argument struct
- `cyto-io` — I/O handle creation

## Testing

```bash
cargo test -p cyto-ibu-umi-correct
```

Unit tests in `src/utils.rs` validate connected components against petgraph's built-in algorithms on random G(n,p) and G(n,m) graphs.
