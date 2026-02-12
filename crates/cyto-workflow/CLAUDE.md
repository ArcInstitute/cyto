# cyto-workflow

## Purpose

Orchestrates end-to-end analysis pipelines. Runs the full sequence: map -> sort -> umi-correct -> reads -> count -> convert -> filter/assign. Parallelizes post-mapping steps across probes using Rayon. Invokes external Python tools (`pycyto`, `cell-filter`, `geomux`) via `std::process::Command`.

## Key Source Files

- `src/gex.rs` — `run()`: GEX workflow entry point. Calls `cyto_map::run_gex2()`, then parallelizes `ibu_steps()` across all per-probe IBU files. Distributes threads proportionally across files.
- `src/crispr.rs` — `run()`: CRISPR workflow entry point. Same structure as GEX but passes `ArgsGeomux` for guide assignment step.
- `src/utils.rs` — Core workflow utilities:
  - `ibu_steps()` — Orchestrates per-IBU pipeline: sort -> umi-correct (optional) -> reads stats (optional) -> count -> h5ad conversion (optional) -> filter/assign. Cleans up intermediate files.
  - `identify_ibu_files()` — Globs `outdir/ibu/*.ibu`, excludes `.sort.ibu`
  - `convert_to_h5ad()` — Calls `pycyto convert`, removes MTX directory on success
  - `filter_h5ad()` — Calls `cell-filter` (EmptyDrops), handles missing filtered output gracefully
  - `assign_guides()` — Calls `geomux` with full parameter passthrough, handles known warning conditions
  - `write_done_file()` / `write_timings_file()` — Writes workflow completion marker and timing TSV
- `src/timing.rs` — `ModuleTiming` (ibu_name, module, elapsed_secs), `Module` enum (Mapping, InitialSort, UmiCorrection, ReadsDump, Counting, ConversionH5ad, DropletFiltering, GuideAssignment)

## Key Types

- `ModuleTiming` — Per-step timing record, serialized to `.timings.tsv`
- `Module` — Enum for each pipeline step
- `RefWorkflowCommand<'a>` — Debug-printable reference to workflow args, written to `.done` file

## Design Notes

- Thread distribution: total threads divided evenly across IBU files, minimum 1 per file
- Intermediate IBU files are removed as they're consumed (unsorted -> sorted -> umi-corrected)
- External tool errors surface as `bail!()` with stdout/stderr logged, except for known warnings (e.g., "No guides passed the cell threshold") which are logged as warnings
- Output directory structure: `ibu/`, `counts/`, `stats/`, `assignments/`, `metadata/`

## Dependencies (within workspace)

- `cyto-map` — Initial mapping step
- `cyto-ibu-sort`, `cyto-ibu-umi-correct`, `cyto-ibu-count`, `cyto-ibu-reads` — Post-mapping IBU processing
- `cyto-cli` — Argument types (`ArgsWorkflow`, `ArgsGeomux`, `ArgsSort`, `ArgsUmi`, `ArgsCount`, `ArgsReads`)

## Testing

```bash
cargo test -p cyto-workflow
```

Integration tests via `justfile`:
```bash
just run-wf-gex
just run-wf-crispr
```
