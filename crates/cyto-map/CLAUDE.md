# cyto-map

## Purpose

Core mapping engine. Maps paired-end sequencing reads to features (genes, CRISPR guides) using sequence hashing. Handles barcode correction, UMI extraction, and optional probe demultiplexing. When probes are present, writes per-probe IBU output files; otherwise writes a single IBU. Supports both BINSEQ and FASTX inputs via parallel processing.

## Key Source Files

- `src/detect.rs` — Geometry detection module. Samples N reads **per input file (lane)** independently, scans all positions for known reference sequences, then reconciles lanes in `finalize_detection`: (1) **pools** all per-lane `PositionAccumulator`s and infers the **reported geometry from the pool** (sharpens estimates for shallow lanes); (2) infers each lane with the accept threshold disabled so weak lanes still yield a result; (3) **accepts on the per-lane maximum** — detection succeeds iff **at least one lane** matches every component at or above `min_proportion` (a single low-signal lane cannot cause failure); (4) checks **cross-lane consistency** by per-component position tolerance against the pool (feature components tolerate the max remap window across pool and lanes — computed before this check — barcode/probe a fixed 2), where only components a lane is *strong* on vote; (5) reports the **maximum remap window** across the pool and all lanes (advisory — the mapper uses `--remap-window`, not this value). A lane is fatal only on zero reads, an unresolvable overlap, or a missing required component (`infer_geometry` bails naming the component instead of panicking). Types: `DetectionConfig` (`#[derive(Clone, Copy)]`; `num_reads`, `min_proportion`, `remap_min_proportion`, `num_threads`), `ComponentEvidence`, `PerFileResult` (per-lane detection summary), `DetectionResult` (includes `per_file_results: Vec<PerFileResult>`, one entry per input lane including weak ones). Public API: `detect_gex_geometry()`, `detect_crispr_geometry()` (both consume unpositioned mappers, sample all input files, pool+accept+validate via `finalize_detection`), `log_detection_result()` (logs aggregated result with per-file counts to stderr via `info!`; the bare geometry string itself is printed to stdout by `run_detect_*` in `run.rs`, not by this module). Internal: `PositionAccumulator`, `GexDetectionProcessor`/`CrisprDetectionProcessor` (implement both `binseq::ParallelProcessor` and `paraseq::PairedParallelProcessor`), `infer_geometry()` (bails on a missing required component), `find_best_positions()` (total tie-break sort so single-lane output and lane-vs-pool comparison are deterministic across independently-seeded HashMaps), `estimate_remap_window()` (proportion-based threshold, contiguous-range walk, total tie-break argmax for determinism across independently-seeded HashMaps), `resolve_overlaps()`, `finalize_detection()` + `component_tolerance()` (pooled estimate, per-lane-max accept, tolerance consistency, advisory max remap window), `log_per_file_result()`, `log_top_alternatives()`.
- `src/geometry.rs` — Geometry DSL parser and resolver. Parses bracket notation (e.g. `[barcode][umi:12][:10][probe] | [gex]`) into `Geometry` → resolves to `ResolvedGeometry` with concrete byte offsets and read mates. `has_component()` checks whether a geometry includes a given component (used for probe validation). Extensive unit tests (~300 lines).
- `src/mapper/mod.rs` — `Mapper` trait (`query(&self, seq) -> Option<usize>`, `mate() -> ReadMate`), `Library` trait (statistics), typestate markers (`Unpositioned`, `Ready`)
- `src/mapper/gex.rs` — `GexMapper<S>`: maps GEX probes via `SplitSeqHash`. Two-half matching with configurable hamming distance (`GEX_MAX_HDIST=3`). Implements `FeatureWriter`.
- `src/mapper/crispr.rs` — `CrisprMapper<S>`: two-stage matching — `MultiLenSeqHash` for variable-length anchors, then `SeqHash` for fixed-length protospacers. Protospacer offset computed dynamically from anchor match.
- `src/mapper/whitelist.rs` — `WhitelistMapper<S>`: cell barcode correction via `SeqHash`. Returns 2-bit encoded barcodes. Supports parallelized hash build.
- `src/mapper/probe.rs` — `ProbeMapper<S>`: demultiplexing probe mapper with optional regex filtering on aliases. Creates `Bijection` for unique probe-to-index mapping.
- `src/mapper/umi.rs` — `UmiMapper`: extracts UMI from reads, validates quality scores against threshold (`UMI_MIN_QUALITY=10`), provides 2-bit encoding.
- `src/mapper/biject.rs` — `Bijection<T>`: bidirectional map (element <-> index) used for deduplicating probe aliases.
- `src/processor.rs` — `MapProcessor<M>`: parallelized read processing. Handles both probed (multi-output with `Option<ProbeMapper>` and `Option<Bijection>`) and unprobed (single-output) modes in a unified struct. Two constructors: `probed()` and `unprobed()`. Implements both `binseq::ParallelProcessor` and `paraseq::PairedParallelProcessor`. Thread-local buffers flushed on batch complete. Progress bar on thread 0.
- `src/run.rs` — `run_gex()` and `run_crispr()`: top-level orchestration. Each internally handles both probed and unprobed inputs. Shared pipeline extracted into generic `run_pipeline<M>()`. Geometry determination via `parse_geometry()`: uses `--preset` if given, then `--geometry`, else falls back to the V1 default for that mode. Standalone detect: `run_detect_gex()` and `run_detect_crispr()` run detection only, log detailed per-component evidence to stderr, print the bare geometry string to stdout for shell composition (`$(...)`), log a `Detected geometry matches preset \`<name>\`` info line on stderr when the detected geometry equals one of the four canonical Flex presets (gex-v1/v2, crispr-v1/v2)`. Helpers: `load_probe()`, `load_detect_probe()`, `validate_probe_geometry()`, `preset_name_for_geometry()` (reverse-lookup from geometry string to preset name; `GEOMETRY_CRISPR_PROPERSEQ` intentionally excluded).
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

Unit tests live in `src/geometry.rs` (parser and resolution) and `src/detect.rs` (position accumulation, `infer_geometry`, remap-window estimation, and `finalize_detection` lane reconciliation). End-to-end detection over the files in `data/` is covered by `tests/detect_integration.rs`. Mapping integration tests use `justfile` targets:

```bash
just run-gex-binseq
just run-crispr-binseq
```
