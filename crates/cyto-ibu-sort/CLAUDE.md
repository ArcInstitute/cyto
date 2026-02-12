# cyto-ibu-sort

## Purpose

Sorts IBU files by the natural record ordering (barcode, umi, index). Supports both in-memory sorting and external (disk-based) sorting for large files. Used as the first post-mapping step in the workflow pipeline.

## Key Source Files

- `src/lib.rs` — Single-file implementation with `run()` entry point:
  - **In-memory mode**: Loads all records into a `Vec`, calls `sort_unstable()`, writes output
  - **External mode**: Uses `ext-sort` crate with configurable memory limit (default 5 GiB). Chunk size calculated as `memory_limit / 24` (IBU record size). Multi-threaded.

## Key Types

- Uses `ibu::Reader` / `ibu::Writer` for IBU I/O
- `ExternalSorter` from `ext-sort` crate with `LimitedBufferBuilder` for memory-bounded sorting

## Design Notes

- IBU records are 24 bytes each (barcode: u64, umi: u64, index: u64)
- External sorting uses `rmp` (MessagePack) serialization for temporary chunks
- Memory limit is parsed from human-readable strings (e.g., "5GiB") via `bytesize`
- Supports stdin/stdout for piped workflows

## Dependencies (within workspace)

- `cyto-cli` — `ArgsSort` argument struct
- `cyto-io` — `match_input()`, `match_output()` for handle creation

## Testing

```bash
cargo test -p cyto-ibu-sort
```

No unit tests — tested through workflow integration tests.
