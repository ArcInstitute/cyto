# cyto-cli

## Purpose

Defines all CLI argument structures using Clap. This crate is a pure definition layer — it contains no processing logic, only argument parsing, validation, and preset constants. All other crates depend on it for their argument types.

## Key Source Files

- `src/commands.rs` — Top-level `Commands` enum (`Workflow`, `Map`, `Ibu`)
- `src/map/mod.rs` — `MapCommand` enum (Gex, Crispr) and geometry preset string constants (`GEOMETRY_GEX_FLEX_V1`, `GEOMETRY_GEX_FLEX_V2`, etc.)
- `src/map/options.rs` — `MapOptions` (geometry DSL, preset selection, exact matching, remap window, auto-detection params: `geometry_auto_num_reads`, `geometry_auto_min_proportion`), `GeometryPreset` enum, `WhitelistOptions`, `ProbeOptions`
- `src/map/input.rs` — `MultiPairedInput` handles both BINSEQ (`.bq`/`.vbq`/`.cbq`) and FASTX paired-end inputs
- `src/map/gex.rs` — `ArgsGex` flattens input, map options, GEX library path, runtime, and output
- `src/map/crispr.rs` — `ArgsCrispr` same structure but with CRISPR guides path
- `src/map/runtime.rs` — `RuntimeOptions` (thread count, verbose flag)
- `src/output.rs` — `ArgsOutput` (output directory, force overwrite, `min_ibu_records` threshold)
- `src/ibu/mod.rs` — `IbuCommand` enum with subcommands (View, Cat, Sort, Count, Umi, Reads) and their `Args*` structs in submodules
- `src/workflow/mod.rs` — `WorkflowCommand`, `ArgsWorkflow` (skip flags, format selection, sort options), `ArgsGeomux` (CRISPR guide assignment params), external tool version constants and `uv` installation logic

## Key Types

- `Commands` — Top-level subcommand routing enum
- `MapCommand` / `IbuCommand` / `WorkflowCommand` — Per-module subcommand enums
- `GeometryPreset` — Enum mapping preset names to geometry DSL strings
- `MultiPairedInput` — Handles BINSEQ vs FASTX input detection and reader creation
- `ArgsWorkflow` — Workflow options including `CountFormat` (H5ad, Mtx, Tsv) and external tool validation via `uv`
- `ArgsGeomux` — CRISPR guide assignment parameters (min UMI thresholds, FDR, log-odds, geomux vs mixture mode)

## Design Notes

- Geometry presets: V2 presets force `remap_window=5`, V1 uses default of 1
- `MultiPairedInput.is_binseq()` auto-detects format by file extension
- `ArgsWorkflow.validate_requirements()` transparently installs Python tools (`pycyto`, `cell-filter`, `geomux`) via `uv tool install` at pinned versions
- External tool versions are pinned as constants: `VERSION_GEOMUX`, `VERSION_CELL_FILTER`, `VERSION_PYCYTO`

## Dependencies (within workspace)

- `cyto-io` — For `validate_output_directory`

## Testing

```bash
cargo test -p cyto-cli
```

No unit tests — this is a definition-only crate. Argument parsing is tested implicitly through integration tests.
