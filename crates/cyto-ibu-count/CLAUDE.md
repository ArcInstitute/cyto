# cyto-ibu-count

## Purpose

Creates barcode-by-feature count matrices from sorted IBU files. Deduplicates UMIs by selecting the most abundant index per UMI (discarding ties), optionally aggregates counts at a higher level (e.g., gene level from probe level), and writes output in TSV, MTX, or h5ad format.

## Key Source Files

- `src/dedup.rs` — UMI deduplication logic:
  - `deduplicate_umis()` — Streams through sorted IBU records, tracking `UmiState` per UMI. For each UMI, selects the index with highest abundance; ties result in the UMI being discarded. Validates sorted input and max index bounds.
  - `UmiState` — State machine tracking current/max index abundance and tie status
  - `BarcodeIndexCounts` — Sparse count matrix as `HashMap<barcode, HashMap<index, count>>`. Tracks nnz (non-zero entries).
  - `BarcodeIndexCount` — (barcode, index, count) tuple for iteration
  - `DeduplicateError` — Custom errors: `UnsortedIbu`, `MaxIndexExceeded`, `EmptyStream`
  - Extensive unit tests (~400 lines) covering deduplication edge cases
- `src/lib.rs` — Output formatting and aggregation:
  - `run()` — Entry point: loads features, deduplicates, optionally aggregates, dispatches to h5ad/mtx/tsv writer based on `ArgsCount.h5ad`/`.mtx`
  - `aggregate_unit()` — Aggregates counts by feature name (e.g., probe -> gene level)
  - `write_counts_tsv()` — TSV output (encoded 2-bit or decoded nucleotide barcodes)
  - `write_counts_mtx()` — Matrix Market format: `matrix.mtx.gz`, `barcodes.tsv.gz`, `features.tsv.gz` (parallel gzip via `gzp`)
  - `load_features()` — Reads feature names from TSV at specified column index
- `src/h5ad.rs` — Native AnnData output:
  - `write_counts_h5ad()` — Builds a CSR sparse matrix directly from `BarcodeIndexCounts` (no MTX intermediate) and writes it as a `.h5ad` file via the `anndata`/`anndata-hdf5` crates. Forces gzip compression (instead of the crate's zstd default) so the file is readable by plain `h5py`/`anndata` without the optional `hdf5plugin` package.

## Key Types

- `BarcodeIndexCounts` — Sparse count matrix with `insert()`, `insert_count()`, `iter_counts()`, `get_nnz()`, `get_num_barcodes()`
- `UmiState` — Per-UMI state machine: `reset()`, `update_index()`, `update_max()`, `has_clear_winner()`
- `DeduplicateError` — Custom error type for deduplication failures

## Design Notes

- Barcodes can be output as 2-bit encoded integers (`--compressed`) or decoded nucleotide strings (default)
- MTX output uses parallel gzip compression via `gzp::ParCompress`
- h5ad output writes directly from the in-memory count table — no MTX staging and no external Python conversion tool (previously `pycyto convert`)
- Barcode suffix support (e.g., `-ProbeA`) for multiplexed experiments
- Feature aggregation happens post-deduplication: probe-level counts are summed to gene-level

## Dependencies (within workspace)

- `cyto-cli` — `ArgsCount` argument struct
- `cyto-io` — I/O handle creation

## External Dependencies

- `anndata` / `anndata-hdf5` — Native h5ad read/write; statically builds `libhdf5` from source via `hdf5-metno-sys` (requires `cmake` at build time, no system libhdf5 needed at runtime)
- `nalgebra-sparse` — CSR/COO sparse matrix construction for h5ad's `X`

## Testing

```bash
cargo test -p cyto-ibu-count
```

Extensive unit tests in `src/dedup.rs` covering: single/multiple barcodes, UMI deduplication with ties, max index validation, sort order validation, empty streams.
