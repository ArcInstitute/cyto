# cyto-io

## Purpose

Shared file I/O abstractions used across the workspace. Provides input/output handle creation with transparent compression support (gzip, zstd via `niffler`), output directory management, and a generic trait for writing feature metadata as TSV.

## Key Source Files

- `src/feature.rs` — `FeatureWriter<'a>` trait: generic serialization of library features to TSV via `csv` crate. Implementors define a `record_stream()` iterator and get `write_to()` for free.
- `src/write.rs` — `open_file_handle()` (creates buffered writer, auto-creates parent dirs), `validate_output_directory()` (checks existence, handles `--force`), `write_features()` (writes features to `metadata/features.tsv`)
- `src/utils.rs` — Input/output handle matchers:
  - `match_input()` / `match_input_transparent()` — File or stdin reader, optional transparent decompression
  - `match_output()` / `match_output_transparent()` — File or stdout writer, auto-detects gzip/zstd by extension
  - `match_output_stderr()` — Falls back to stderr instead of stdout

## Key Types

- `FeatureWriter<'a>` trait — Requires `record_stream() -> impl Iterator<Item = Record>` where `Record: Serialize`. Provides default `write_to()` that writes tab-delimited headerless TSV.

## Dependencies (within workspace)

None — this is a leaf crate. Depends on `niffler` for compression, `csv` for TSV, `serde` for serialization.

## Testing

```bash
cargo test -p cyto-io
```

No unit tests — functions are tested implicitly through downstream crate tests.
