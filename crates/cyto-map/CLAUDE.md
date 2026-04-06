# cyto-map

## Purpose

Core mapping engine. Maps paired-end sequencing reads to features (genes, CRISPR guides) using sequence hashing. Handles barcode correction, UMI extraction, and optional probe demultiplexing. When probes are present, writes per-probe IBU output files; otherwise writes a single IBU. Supports both BINSEQ and FASTX inputs via parallel processing.

## Key Source Files

- `src/detect.rs` — Geometry auto-detection module. Samples N reads from input, scans all positions for known reference sequences using `query_sliding_iter`, infers optimal geometry from position consensus. Types: `DetectionConfig`, `ComponentEvidence`, `DetectionResult`. Public API: `detect_gex_geometry()`, `detect_crispr_geometry()` (both consume unpositioned mappers). Internal: `PositionAccumulator`, `GexDetectionProcessor`/`CrisprDetectionProcessor` (implement both `binseq::ParallelProcessor` and `paraseq::PairedParallelProcessor`), `infer_geometry()`, `estimate_remap_window()`, `resolve_overlaps()`.
- `src/geometry.rs` — Geometry DSL parser and resolver. Parses bracket notation (e.g. `[barcode][umi:12][:10][probe] | [gex]`) into `Geometry` → resolves to `ResolvedGeometry` with concrete byte offsets and read mates. `has_component()` checks whether a geometry includes a given component (used for probe validation). Extensive unit tests (~300 lines).
- `src/mapper/mod.rs` — `Mapper` trait (`query(&self, seq) -> Option<usize>`, `mate() -> ReadMate`), `Library` trait (statistics), typestate markers (`Unpositioned`, `Ready`)
- `src/mapper/gex.rs` — `GexMapper<S>`: maps GEX probes via `SplitSeqHash`. Two-half matching with configurable hamming distance (`GEX_MAX_HDIST=3`). Implements `FeatureWriter`.
- `src/mapper/crispr.rs` — `CrisprMapper<S>`: two-stage matching — `MultiLenSeqHash` for variable-length anchors, then `SeqHash` for fixed-length protospacers. Protospacer offset computed dynamically from anchor match.
- `src/mapper/whitelist.rs` — `WhitelistMapper<S>`: cell barcode correction via `SeqHash`. Returns 2-bit encoded barcodes. Supports parallelized hash build.
- `src/mapper/probe.rs` — `ProbeMapper<S>`: demultiplexing probe mapper with optional regex filtering on aliases. Creates `Bijection` for unique probe-to-index mapping.
- `src/mapper/umi.rs` — `UmiMapper`: extracts UMI from reads, validates quality scores against threshold (`UMI_MIN_QUALITY=10`), provides 2-bit encoding.
- `src/mapper/biject.rs` — `Bijection<T>`: bidirectional map (element <-> index) used for deduplicating probe aliases.
- `src/processor.rs` — `MapProcessor<M>`: parallelized read processing. Handles both probed (multi-output with `Option<ProbeMapper>` and `Option<Bijection>`) and unprobed (single-output) modes in a unified struct. Two constructors: `probed()` and `unprobed()`. Implements both `binseq::ParallelProcessor` and `paraseq::PairedParallelProcessor`. Thread-local buffers flushed on batch complete. Progress bar on thread 0.
- `src/run.rs` — `run_gex()` and `run_crispr()`: top-level orchestration. Each internally handles both probed and unprobed inputs. Shared pipeline extracted into generic `run_pipeline<M>()`. Geometry determination: when `--preset` or `--geometry` is given, parses directly; otherwise triggers auto-detection via `autodetect_gex_geometry()`/`autodetect_crispr_geometry()` which consume unpositioned mappers (requiring a reload for the mapping pipeline). Helpers: `load_probe_with_window()`, `validate_probe_geometry()` (errors if geometry has `[probe]` without a probe file, warns if probe file given without `[probe]` in geometry), `log_detection_result()`.
- `src/stats.rs` — `MappingStatistics`, `UnmappedStatistics`, `LibraryStatistics`, `InputRuntimeStatistics`. JSON serialization to `stats/` directory.
- `src/utils.rs` — `build_filepaths()`, `initialize_output_ibus()` (writes IBU headers), `delete_sparse_ibus()` (removes IBU files below record threshold).

## Key Types and Traits

- **Traits**: `Mapper` (sequence query), `Library` (statistics), `FeatureWriter` (TSV output)
- **Mappers**: `GexMapper<S>`, `CrisprMapper<S>`, `WhitelistMapper<S>`, `ProbeMapper<S>`, `UmiMapper`
- **Geometry**: `Component` enum, `Region` enum (Component or Skip), `Geometry`, `ResolvedGeometry`, `ResolvedRegion`
- **Processing**: `MapProcessor<M>` — generic over any `Mapper` implementation
- **Utilities**: `Bijection<T>`, `MappingStatistics`, `UnmappedStatistics`

## Design Patterns

- **Typestate**: All mappers use `Unpositioned` → `Ready` typestate. `from_file()` returns `<Unpositioned>`, then `.resolve(&geometry)` returns `<Ready>`. Only `<Ready>` implements `Mapper`.
- **Two-phase geometry**: Geometry is parsed from DSL, then resolved by querying each mapper for its sequence length. Variable-length components (e.g. anchor) get `None` length.
- **Optional probe demultiplexing**: Probe fields (`probe_mapper`, `bijection`) are `Option`-wrapped in `MapProcessor`. When `None`, output goes to a single writer; when `Some`, output is routed to per-probe writers via the bijection index.
- **Thread-local batching**: `MapProcessor` accumulates IBU records in per-thread buffers (`t_output`), flushing to shared mutex-protected writers on batch complete. Statistics are similarly accumulated locally then merged.
- **Dual input support**: `process_input()` handles both BINSEQ (via `binseq::ParallelReader`) and FASTX (via `paraseq`) through different trait impls on the same `MapProcessor`.

## Dependencies (within workspace)

- `cyto-cli` — Argument types (`ArgsGex`, `ArgsCrispr`, `MultiPairedInput`)
- `cyto-io` — File handles, `FeatureWriter` trait, `write_features()`

## Testing

```bash
cargo test -p cyto-map
```

Unit tests are in `src/geometry.rs` (parser and resolution tests). Integration tests use `justfile` targets:

```bash
just run-gex-binseq
just run-crispr-binseq
```
