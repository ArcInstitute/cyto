# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- `cyto workflow gex`/`crispr` (and `cyto ibu count --h5ad`) now write `.h5ad` output
  natively from Rust instead of staging through MTX and shelling out to `pycyto convert`.
  `cyto-ibu-count` builds the sparse count matrix directly from its in-memory dedup
  table and writes it via the `anndata`/`anndata-hdf5` crates, so h5ad output no longer
  round-trips through on-disk MTX text files or spawns a Python subprocess.
- `cell-filter` (GEX droplet filtering) and `geomux` (CRISPR guide assignment) are
  unchanged and still run as external Python tools via `uv`, invoked on the natively
  written `.h5ad`.
- `pycyto` is no longer installed or invoked anywhere in `cyto workflow`. It remains
  useful standalone for aggregating multiple `cyto` outputs into one `h5ad`
  (`pycyto aggregate`), documented in the README.
- The h5ad `X` matrix is now stored as unsigned 32-bit integers rather than the
  float32 that `pycyto convert` produced by default. Both `cell-filter` and `geomux`
  already cast counts back to `int` internally, so this has no behavioral effect
  downstream.

### Added

- `cyto ibu count --h5ad`: writes a native AnnData `.h5ad` file directly (mutually
  exclusive with `--mtx`).
- New `cyto-ibu-count` dependencies: `anndata`, `anndata-hdf5` (statically builds
  `libhdf5` from source via `hdf5-metno-sys`, requires `cmake` at build time — no
  system `libhdf5` needed at runtime), and `nalgebra-sparse`.

### Fixed

- A file-locking race: `cyto-workflow` processes probes concurrently via Rayon, and
  HDF5's advisory file locking could spuriously report a just-written, already-closed
  `.h5ad` as locked while a sibling probe's `.h5ad` was still open elsewhere in the
  same process. `cell-filter`/`geomux` subprocess invocations now set
  `HDF5_USE_FILE_LOCKING=FALSE` to avoid this.

### Removed

- `convert_to_h5ad()` and the `pycyto` external-tool install path
  (`VERSION_PYCYTO`, the `uv tool install pycyto` step in
  `ArgsWorkflow::validate_requirements()`).
- `Module::ConversionH5ad` timing entry (h5ad write time is now folded into the
  `Counting` step) and `ArgsWorkflow::mtx()` (no longer needed now that h5ad output
  doesn't stage through MTX).
