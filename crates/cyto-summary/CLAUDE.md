# cyto-summary

## Purpose

Generates self-contained HTML QC reports from finished cyto output directories,
analogous in spirit to Cell Ranger's `web_summary.html` but built entirely from
the machine-readable stat files each pipeline stage already writes. No data is
recomputed from reads/IBUs -- the command only aggregates and renders existing
`stats/` artifacts, so it adds negligible runtime.

Invoked as `cyto summary <dir>...`. Each input may be a single sample directory
(one containing `stats/`) or a parent directory holding several; sample dirs are
auto-discovered. Output: one sample-prefixed `<sample>.cyto-report.html` per
sample, a `<run>.cyto-report.html` master index when more than one sample is
present, and a `<sample>.cyto-report.json` machine-readable sidecar per sample
(suppressed with `--no-json`). Reports default to `<common-parent>/cyto_reports`.

## Key Source Files

- `src/lib.rs` — `run(&ArgsSummary)` entry point. Discovers samples, resolves the
  output directory and run name, collects each sample, renders per-sample HTML +
  JSON, and writes the master index. Helpers: `sanitize()` (filename-safe names),
  `common_parent()`.
- `src/model.rs` — Data model. `*Raw` structs (`MappingRaw`, `UnmappedRaw`,
  `LibraryRaw`, `RunRaw`, `UmiRaw`, `AssignmentRaw`, `TimingRow`) mirror the
  on-disk JSON/TSV stat files (deserialization targets). Report structs
  (`SampleReport`, `ProbeSummary`, `AssignmentSummary`, `KneePoint`, `SampleKind`)
  are the aggregated, serialized view rendered into HTML.
- `src/collect.rs` — `collect_sample()` builds a `SampleReport` from a directory.
  Best-effort: missing/malformed stat files degrade a panel and record a note
  rather than failing. `discover_samples()`/`is_sample_dir()` handle input
  expansion; `read_reads()` parses `reads.tsv.zst` (via `cyto_io::match_input_transparent`)
  into totals, medians, and the log-spaced barcode-rank `knee_points()`;
  `detect_kind()` classifies GEX vs CRISPR from `.done`/stats subdirs.
  `read_h5ad_seen()` (gated by `read_features`, i.e. not `--no-features`) opens the
  count matrix (`counts/{probe}.filt.h5ad` preferred, else `.h5ad`) via the `hdf5`
  crate and streams the CSR column indices in chunks to count distinct
  features detected; per-probe counts and the cross-probe union land in
  `ProbeSummary::features_detected` and `SampleReport::features_{total,detected}`.
- `src/render.rs` — HTML rendering. Inlined CSS (`CSS`, a white "printout" theme),
  hand-built SVG barcode-rank plot (`knee_svg`), and section builders
  (`sample_metrics`, `cells_section`, `mapping_section`, `probe_section`,
  `moi_section`, `timing_section`, `library_section`, `alerts`). Entry points:
  `render_sample()` and `render_master()`. Formatting/layout helpers: `int`,
  `compact`, `pct`, `f1`, `esc`, `metric`, `eyebrow`, `bar`, `page`, `footer`.

## Design Notes

- **No new heavy compute**: the report is an aggregation/presentation layer over
  existing `stats/` files. The only derived value from bulk data is the endpoint
  sequencing saturation (`1 - UMIs/reads`) and per-barcode medians/knee, all read
  in a single streaming pass over `reads.tsv.zst`.
- **Self-contained output**: all CSS inlined, plots emitted as inline SVG (no JS,
  no external assets, no plotting dependency). A report is one portable file, KB-
  sized rather than the tens of MB a Cell Ranger `web_summary.html` embeds.
- **Visual style**: a flat white "sequencing QC printout" -- hairline rules rather
  than filled cards, all numeric data in tabular monospace, one restrained accent,
  eyebrow section labels. Layout order follows what single-cell users know from
  Cell Ranger (headline metrics -> metrics-left / barcode-rank-right -> mapping ->
  per-probe) without copying its styling. Light theme only (no system fonts fetched).
- **Robustness**: every panel is guarded. A sample missing `stats/filtering` or a
  malformed JSON still produces a report; issues surface in the Notes card.
- Metrics richer than Cell Ranger where cyto has them: per-reason unmapped
  breakdown (missing probe/feature/whitelist, failed UMI quality) and UMI
  correction rate.

## Dependencies (within workspace)

- `cyto-cli` — `ArgsSummary`
- `cyto-io` — `match_input_transparent` for transparent `.zst` reads

## External dependencies

- `hdf5` (`hdf5-metno`) + `ndarray` for reading count-matrix `h5ad` files. This
  links **system libhdf5** (found via `pkg-config`, e.g. `libhdf5-dev`) — building
  cyto now requires it. Only the genes/guides-detected metrics use it; `--no-features`
  skips the h5ad read but the link dependency remains.

## Testing

```bash
cargo test -p cyto-summary
```

Unit tests in `src/collect.rs` cover `median_sorted`, `saturation`, and
`knee_points` (monotonicity, bounds, small-input passthrough).
