# cyto-ibu-cat

## Purpose

Concatenates multiple IBU files into a single output file. Validates that all input files have matching headers (same barcode length, UMI length, etc.) before merging.

## Key Source Files

- `src/lib.rs` — Single-file implementation (~50 lines):
  - `run()` — Opens all input IBU files, validates headers match, writes all records sequentially to output

## Design Notes

- Header validation ensures consistency: all inputs must have the same barcode/UMI lengths
- Simple sequential concatenation — does not sort the output
- Supports stdout output for piping

## Dependencies (within workspace)

- `cyto-cli` — `ArgsCat` argument struct
- `cyto-io` — `match_output()` for handle creation

## Testing

```bash
cargo test -p cyto-ibu-cat
```

No unit tests — simple utility crate.
