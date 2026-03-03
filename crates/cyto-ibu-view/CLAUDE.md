# cyto-ibu-view

## Purpose

Dumps IBU file contents as human-readable text for debugging and inspection. Outputs header metadata as comments and records as tab-delimited rows. Supports both raw 2-bit encoded output and decoded nucleotide sequences. Optionally annotates records with feature names.

## Key Source Files

- `src/lib.rs` — Single-file implementation:
  - `run()` — Entry point: reads IBU, optionally writes header, dumps records
  - `write_header()` — Outputs IBU metadata (version, barcode_len, umi_len, sorted flag) as `# ` comment lines
  - `dump_encoded_records()` — Outputs raw u64 values for barcode, umi, index
  - `dump_decoded_records()` — Decodes 2-bit barcode and UMI to nucleotide strings via `bitnuc`
  - `load_features()` — Optionally loads feature names to replace numeric indices

## Design Notes

- Header-only mode (`--header`) for quick metadata inspection
- Feature annotation replaces numeric index column with feature name from TSV file
- Reuses barcode/UMI decode buffers across records for efficiency

## Dependencies (within workspace)

- `cyto-cli` — `ArgsView` argument struct
- `cyto-io` — I/O handle creation

## Testing

```bash
cargo test -p cyto-ibu-view
```

No unit tests — utility crate tested through manual inspection and integration tests.
