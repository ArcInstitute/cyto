# cyto-ibu-reads

## Purpose

Calculates per-barcode read and UMI counts from sorted IBU files. Used for sequencing saturation analysis. Outputs a TSV with columns: barcode, n_umis, n_reads. Supports optional barcode whitelist filtering.

## Key Source Files

- `src/lib.rs` — Single-file implementation:
  - `run()` — Entry point: opens IBU, loads optional whitelist, processes records, writes TSV
  - `process_records()` — Iterates sorted IBU records, counts reads and unique UMIs per barcode. Validates sort order. Skips barcodes not in whitelist.
  - `Whitelist` — Optional `HashSet<u64>` of 2-bit encoded barcodes loaded from a text file. When `None`, all barcodes pass.
  - `Stats<T>` — Generic per-barcode statistics struct (barcode, n_umis, n_reads). Supports both encoded (u64) and decoded (nucleotide string) barcode output.

## Key Types

- `Stats<T>` — Serializable per-barcode stats, generic over barcode type
- `Whitelist` — Optional barcode filter with `from_path()` and `matches()` methods

## Design Notes

- Expects sorted IBU input (validates ordering, bails on unsorted)
- Whitelist barcodes are stored as 2-bit encoded u64 for efficient lookup
- Output supports transparent compression (gzip/zstd) based on file extension via `match_output_transparent()`

## Dependencies (within workspace)

- `cyto-cli` — `ArgsReads` argument struct
- `cyto-io` — I/O handle creation with transparent compression

## Testing

```bash
cargo test -p cyto-ibu-reads
```

No unit tests — tested through workflow integration tests.
