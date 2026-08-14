# cyto-cli

## Purpose

Defines all CLI argument structures using Clap. This crate is a pure definition layer — it contains no processing logic, only argument parsing, validation, and preset constants. All other crates depend on it for their argument types.

## Key Source Files

- `src/commands.rs` — Top-level `Commands` enum (`Workflow`, `Map`, `Detect`, `Ibu`)
- `src/map/mod.rs` — `MapCommand` enum (Gex, Crispr) and geometry preset string constants (`GEOMETRY_GEX_FLEX_V1`, `GEOMETRY_GEX_FLEX_V2`, etc.)
- `src/map/options.rs` — `MapOptions` (geometry DSL, preset selection, exact matching, remap window), `GeometryPreset` enum, `WhitelistOptions`, `ProbeOptions`
- `src/map/input.rs` — `MultiPairedInput` handles both BINSEQ (`.bq`/`.vbq`/`.cbq`) and FASTX paired-end inputs
- `src/map/gex.rs` — `ArgsGex` flattens input, map options, GEX library path, runtime, and output
- `src/map/crispr.rs` — `ArgsCrispr` same structure but with CRISPR guides path
- `src/map/runtime.rs` — `RuntimeOptions` (thread count, verbose flag)
- `src/output.rs` — `ArgsOutput` (output directory, force overwrite, `min_ibu_records` threshold)
- `src/ibu/mod.rs` — `IbuCommand` enum with subcommands (View, Cat, Sort, Count, Umi, Reads) and their `Args*` structs in submodules
- `src/detect/mod.rs` — `DetectCommand` enum (Gex, Crispr), `ArgsDetectGex`, `ArgsDetectCrispr`, `DetectionOptions` (fields: `num_threads`, `num_reads`, `min_proportion`, `remap_min_proportion`; `num_threads()` accessor resolves `0 → num_cpus::get()` mirroring `RuntimeOptions`). Flattens `WhitelistOptions`, `ProbeOptions`, `GexOptions`/`CrisprOptions` from `map/` -- no `MapOptions`, `ArgsOutput`, or `RuntimeOptions`.
- `src/workflow/mod.rs` — `WorkflowCommand`, `ArgsWorkflow` (skip flags, format selection, sort options), `ArgsGeomux` (CRISPR guide assignment params), external tool version constants, and `uvx` invocation logic (`uvx_command` helper)

## Key Types

- `Commands` — Top-level subcommand routing enum
- `MapCommand` / `DetectCommand` / `IbuCommand` / `WorkflowCommand` — Per-module subcommand enums
- `GeometryPreset` — Enum mapping preset names to geometry DSL strings
- `MultiPairedInput` — Handles BINSEQ vs FASTX input detection and reader creation
- `ArgsWorkflow` — Workflow options including `CountFormat` (H5ad, Mtx, Tsv) and external tool validation via `uvx`
- `ArgsGeomux` — CRISPR guide assignment parameters (min UMI thresholds, FDR, log-odds, geomux vs mixture mode)

## Design Notes

- Geometry presets: V2 presets force `remap_window=5`, V1 uses default of 1
- `MultiPairedInput.is_binseq()` auto-detects format by file extension
- `ArgsWorkflow.validate_requirements()` runs Python tools (`pycyto`, `cell-filter`, `geomux`) via `uvx` at pinned versions in an ephemeral environment, without mutating the user's global `uv tool` set. It checks `uvx` is on `$PATH` and warms each tool's ephemeral environment once up front.
- `uvx_command(name, version)` builds a `uvx --from '{name}=={version}' {name}` command; callers append the tool's own subcommand and args. Used by `validate_requirements()` and by `cyto-workflow`'s invocation sites.
- External tool versions are pinned as constants: `VERSION_GEOMUX`, `VERSION_CELL_FILTER`, `VERSION_PYCYTO`

## Dependencies (within workspace)

- `cyto-io` — For `validate_output_directory`

## Testing

```bash
cargo test -p cyto-cli
```

No unit tests — this is a definition-only crate. Argument parsing is tested implicitly through integration tests.
