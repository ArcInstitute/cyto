# cyto

[![BSD3-Clause](https://img.shields.io/badge/license-BSD3-blue.svg)](./LICENSE.md)
[![Crates.io](https://img.shields.io/crates/d/cyto?color=orange&label=crates.io)](https://crates.io/crates/cyto)

Ultra-high throughput processing for 10x Genomics Flex single-cell sequencing.

## Overview

`cyto` is a fast, memory-efficient processor for 10x Genomics Flex single-cell RNA sequencing data, designed specifically for production-scale analysis. It handles:

- **Gene expression profiling** from FFPE samples and fresh tissue
- **Highly multiplexed experiments** (16-plex Flex-V1 / 384-plex Flex-V2)
- **CRISPR perturbation screens** (Perturb-seq) with efficient guide assignment
- **Probe-based multiplexing** for clinical and archived samples

`cyto` achieves dramatic performance improvements through algorithmic innovations optimized for Flex's fixed sequence geometry, making previously prohibitive experiments computationally feasible.

### Key Features

- **Ultra-fast processing**: Processes 320k-cell datasets in minutes rather than hours
- **Memory efficient**: Runs on smaller cloud instances with reduced resource requirements
- **Highly accurate**: 99.85% concordance with standard CellRanger outputs, identical cell clustering
- **Modular architecture**: Independent, composable tools for flexible workflows
- **Production-ready**: Built for atlas-scale projects and genome-wide screens
- **[BINSEQ](https://github.com/arcinstitute/binseq) support**: Efficient binary format for highly parallel sequence parsing
- **Compact [IBU format](https://github.com/noamteyssier/ibu)**: Binary Index-Barcode-UMI storage for efficient read processing

## Installation

> Note:
> This crate makes use of SIMD instructions for improved performance.
> To make sure you take advantage of SIMD instructions on your machine set the following environment variable before compiling:

Install via cargo:

```bash
export RUSTFLAGS="-C target-cpu=native";
cargo install cyto
```

Or from source:

```bash
git clone https://github.com/arcinstitute/cyto
cd cyto

# install with cargo
export RUSTFLAGS="-C target-cpu=native"
cargo install --path crates/cyto

# or with just
just install
```

## Quick Start

### Gene Expression Workflow

Process Flex gene expression data with probe demultiplexing:

```bash
cyto workflow gex \
    -c gene_probes.tsv \
    -w cell_barcode_whitelist.txt \
    -p probe_barcodes.txt \
    -o output_dir \
    sample.vbq
```

Or without probe demultiplexing (single-plex or custom geometry):

```bash
cyto workflow gex \
    -c gene_probes.tsv \
    -w cell_barcode_whitelist.txt \
    --geometry "[barcode][umi:12] | [gex]" \
    -o output_dir \
    sample.vbq
```

### CRISPR Screen Workflow

Process Perturb-seq data with guide assignment:

```bash
cyto workflow crispr \
    -c guide_library.tsv \
    -w cell_barcode_whitelist.txt \
    -p probe_barcodes.txt \
    -o output_dir \
    sample.vbq
```

Or without probe demultiplexing:

```bash
cyto workflow crispr \
    -c guide_library.tsv \
    -w cell_barcode_whitelist.txt \
    --geometry "[barcode][umi:12] | [:26][anchor][protospacer]" \
    -o output_dir \
    sample.vbq
```

Both workflows automatically handle:

- Read mapping to features (with integrated barcode correction)
- UMI deduplication
- Molecule counting
- Guide assignment (CRISPR mode)

### Output Structure

Workflows generate organized outputs. With probe demultiplexing, one IBU and count file is generated per probe:

```
output_dir/
├── metadata/
│   └── features.tsv         # Feature index
├── stats/
│   └── mapping.json         # Mapping statistics
├── ibu/
│   ├── probe1.sort.ibu      # Processed IBU files
│   └── probe2.sort.ibu      # (one per probe)
└── counts/
    ├── probe1.counts.tsv    # Count matrices
    └── probe2.counts.tsv    # (one per probe)
```

Without probes, a single IBU and count file is generated:

```
output_dir/
├── metadata/
│   └── features.tsv         # Feature index
├── stats/
│   └── mapping.json         # Mapping statistics
├── ibu/
│   └── output.sort.ibu      # Single IBU file
└── counts/
    └── output.counts.tsv    # Single count matrix
```

## Input Formats

### Feature Libraries

**Gene Expression** (`-c` flag) - 3-column TSV, no header:

```
ENSG00000000003    TSPAN6       ACGTACGTACGTACGT
ENSG00000000005    TNMD         TGCATGCATGCATGCA
```

Columns: Gene ID | Gene Name | Probe Sequence

**CRISPR Guides** (`-c` flag) - 3-column TSV, no header:

```
gene1_guide1    GGGGCCCC    ACGTACGTACGTACGTACGT
gene1_guide2    GGGGCCCC    TGCATGCATGCATGCATGCA
```

Columns: Guide Name | Anchor Sequence | Protospacer Sequence

### Probe Barcodes (Optional)

For multiplexed experiments (`-p` flag) - 3-column TSV, no header:

```
ACGTACGT    BC001    ProbeSet1
TGCATGCA    BC002    ProbeSet2
```

Columns: True Sequence | Alias | Probe Name

**Note**: Probe sequences should match those provided by 10x Genomics for your specific chemistry.

### Cell Barcode Whitelist

Standard 10x barcode whitelist (`-w` flag):

```bash
# Example: 737K barcode list for GEM-X
-w 737K-fixed-rna-profiling.txt.gz
```

### Sequence Files

`cyto` accepts both FASTQ and BINSEQ formats:

```bash
# BINSEQ (recommended - faster parsing)
cyto workflow gex -c probes.tsv -w whitelist.txt sample.vbq

# FASTQ paired-end
cyto workflow gex -c probes.tsv -w whitelist.txt sample_R1.fastq.gz sample_R2.fastq.gz
```

If you have a large collection of sequence files that can be processed as a single input you can provide them all on the CLI:

```bash
# BINSEQ
cyto workflow gex -c probes.tsv -w whitelist.txt *.vbq

# FASTQ paired-end
cyto workflow gex -c probes.tsv -w whitelist.txt *.fastq.gz
```

## Advanced Usage

### Geometry DSL

`cyto` uses a flexible and simple Domain-Specific Language (DSL) for specifying read geometries.
This allows you to describe custom experimental designs that differ from the standard 10x sequence structures.

#### Quick Start: Using Presets

For standard 10x chemistries, use the `--preset` flag:

```bash
# Standard GEX (Flex V1)
cyto workflow gex --preset gex-v1 ...

# GEX with probe on R1 (Flex V2)
cyto workflow gex --preset gex-v2 ...

# CRISPR (standard)
cyto workflow crispr --preset crispr-v1 ...
```

Available presets:
| Preset | Geometry |
|--------|----------|
| `gex-v1` | `[barcode][umi:12] \| [gex][:18][probe]` |
| `gex-v2` | `[barcode][umi:12][:10][probe] \| [gex]` |
| `crispr-v1` | `[barcode][umi:12] \| [probe][anchor][protospacer]` |
| `crispr-v2` | `[barcode][umi:12][:10][probe] \| [:14][anchor][protospacer]` |

> Note: White space is allowed between components and separators.

#### Custom Geometries

For non-standard designs, use the `--geometry` flag with a DSL string:

```bash
cyto workflow gex --geometry "[barcode][umi:12]|[gex][:18][probe]" ...
```

#### DSL Syntax

A geometry string describes the structure of paired-end reads (R1 and R2), separated by `|`:

```
[R1 regions]|[R2 regions]
```

**Components** are functional elements enclosed in brackets:

| Component       | Alias  | Description                 | Length                      |
| --------------- | ------ | --------------------------- | --------------------------- |
| `[barcode]`     | `[bc]` | Cell barcode                | Inferred from whitelist     |
| `[umi:N]`       | —      | Unique Molecular Identifier | Required (e.g., `[umi:12]`) |
| `[probe]`       | —      | Flex probe barcode          | Inferred from probe file    |
| `[gex]`         | —      | Gene expression sequence    | Inferred from library       |
| `[anchor]`      | —      | CRISPR anchor sequence      | Inferred from guide library |
| `[protospacer]` | —      | CRISPR protospacer          | Inferred from guide library |

**Skip regions** are anonymous spacers with explicit lengths:

```
[:N]    # Skip N bases (e.g., [:18] skips 18 bases)
```

#### Examples

**Standard GEX with Flex probe:**

```
[barcode][umi:12]|[gex][:18][probe]
```

- R1: 16bp barcode, 12bp UMI
- R2: Gene expression sequence, 18bp spacer, 8bp probe

**GEX V2 (probe on R1):**

```
[barcode][umi:12][:10][probe]|[gex]
```

- R1: 16bp barcode, 12bp UMI, 10bp spacer, 8bp probe
- R2: Gene expression sequence

**CRISPR with custom spacing:**

```
[barcode][umi:12]|[:20][probe][:6][anchor][protospacer]
```

- R1: 16bp barcode, 12bp UMI
- R2: 20bp offset, probe, 6bp spacer, anchor, protospacer

#### Validation Rules

- Each component can only appear once across both reads
- `[umi:N]` **requires** an explicit length
- Other components infer their lengths from reference files
- Skip regions must have length > 0
- Whitespace between brackets is ignored

#### Remap Window

The `--remap-window` flag controls position tolerance for element matching:

```bash
# Allow +/- 2 position adjustment on failed matches
cyto workflow gex --geometry "..." --remap-window 2 ...

# Exact positions only
cyto workflow gex --geometry "..." --remap-window 0 ...
```

> **Note**: Default is 1. For V2 presets, this is automatically set to 5.

> **Pro-Tip**: If you're unsure about spacer lengths for your library, use [`bqtools grep`](https://github.com/arcinstitute/bqtools?tab=readme-ov-file#grep) to visualize your sequences:
>
> ```bash
> bqtools grep <input.vbq> <anchor_sequence> <probe_sequence>
> ```
>
> This highlights the sequences in your reads so you can count the bases between them.

### Modular Pipeline

For advanced users, `cyto` exposes individual processing steps:

```bash
# 1. Map reads to features (includes barcode correction)
# With probe demultiplexing:
cyto map gex --preset gex-v1 -c probes.tsv -p probe_barcodes.txt -w whitelist.txt -o map_out sample.vbq
# Or without probes:
cyto map gex --geometry "[barcode][umi:12] | [gex]" -c probes.tsv -w whitelist.txt -o map_out sample.vbq

# 2. Sort IBU files
cyto ibu sort -i map_out/ibu/probe1.ibu -o probe1.sorted.ibu

# 3. Correct UMIs
cyto ibu umi -i probe1.sorted.ibu -o probe1.umi.ibu

# 4. Count molecules
cyto ibu count -i probe1.umi.ibu -f map_out/metadata/features.tsv -o counts.tsv
```

This modular design allows:

- Custom processing pipelines
- Integration with orchestration tools (Snakemake, Nextflow)
- Independent scaling of pipeline components
- Checkpointing and resumption

### Multi-threading

Control parallelization with `-T`:

```bash
# Use all available cores
cyto workflow gex --preset gex-v1 -c probes.tsv -w whitelist.txt -T0 sample.vbq

# Use specific number of threads
cyto workflow gex --preset gex-v1 -c probes.tsv -w whitelist.txt -T32 sample.vbq

# Single-threaded (minimal resources)
cyto workflow gex --preset gex-v1 -c probes.tsv -w whitelist.txt -T1 sample.vbq
```

Default: All available threads

### Output Formats

#### TSV Format (default)

Tab-separated sparse matrix:

```
barcode    feature    count
ACGTACGT   ENSG00000000003   5
ACGTACGT   ENSG00000000005   12
```

#### Matrix Market Format

For downstream analysis with scanpy/Seurat:

```bash
cyto ibu count -i sample.ibu -f features.tsv -o counts_mtx --format mtx
```

Generates:

- `matrix.mtx` - Sparse count matrix
- `barcodes.tsv` - Cell barcodes
- `features.tsv` - Feature names

#### Convert to h5ad

Use [pycyto](https://github.com/arcinstitute/pycyto) utilities for format conversion and aggregation:

```bash
# Convert MTX to h5ad
pycyto mtx-to-h5ad counts_mtx/ output.h5ad

# Aggregate cyto output into a single h5ad per sample
pycyto aggregate <config>.json <cyto_output_dir> <aggr_dir>
```

## Guide Assignment (Perturb-seq)

The CRISPR workflow includes automatic guide assignment using the [geomux](https://github.com/noamteyssier/geomux) algorithm, which:

- Scales linearly with data sparsity (not total dimensions)
- Handles multi-guide perturbations
- Works on unfiltered cells (no pre-filtering needed)
- Performs hypergeometric testing with FDR correction

Guide assignments are included in the count matrix output.

## Performance Considerations

`cyto` is optimized for:

- **Fixed-geometry protocols**: Flex libraries with predetermined sequence structures
- **Multiplexed datasets**: Efficient probe demultiplexing at scale
- **Large-scale screens**: Million-cell perturbation experiments

`cyto` is **not** designed for:

- Splice-aware alignment (use STAR, kallisto|bustools, Alevin-fry)
- Transcript discovery or quantification
- Variable read architectures
- Full-length transcript sequencing

## Software Availability

All components are available under the MIT license:

- **cyto**: https://github.com/arcinstitute/cyto
- **pycyto utilities**: https://github.com/arcinstitute/pycyto
- **geomux**: https://github.com/noamteyssier/geomux
- **cell-filter**: https://github.com/arcinstitute/cell-filter
- **IBU format**: https://github.com/noamteyssier/ibu

Rust packages on crates.io | Python packages on PyPI

## Citation

If you use `cyto` in your research, please cite our [BioRxiv preprint](https://www.biorxiv.org/content/10.64898/2026.01.21.700936v1):

```
Teyssier, N. and Dobin, A. (2025). cyto: ultra high-throughput processing
of 10x-flex single cell sequencing. bioRxiv.
```

## Support

- **Issues**: https://github.com/arcinstitute/cyto/issues
- **Documentation**: See `--help` for any command
- **Examples**: See `justfile` for complete workflows

## Acknowledgements

Developed at Arc Institute with support for computational resources.
