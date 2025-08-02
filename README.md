# cyto

A command-line tool for processing structured single-cell data.

## Overview

`cyto` is designed to efficiently process single-cell sequencing data that follows fixed sequence geometries.
It handles sequencing data where:

- Read 1 (R1) contains a fixed structure with:
  - Cell barcode of known length
  - UMI (Unique Molecular Identifier) of known length

- Read 2 (R2) contains target sequences with predictable structure

While cyto can be extended for any single-cell protocol with fixed read geometry, it includes built-in support for:

1. CRISPR screens (with anchor + protospacer structure)
2. GEX systems
3. Flex Probe-based multiplexing

### Key Features

- Fast parsing of structured single-cell data
- [BINSEQ](https://github.com/arcinstitute/binseq) file support
- Efficient [IBU](https://github.com/noamteyssier/ibu) binary storage format (IBU: Index-Barcode-UMI)
- UMI-aware molecule counting
- Flexible mapping strategies for different sequence geometries
- Support for multiplexed [FLEX](https://www.10xgenomics.com/products/flex-gene-expression) experiments via probe sequences

## Installation

`cyto` can be installed via cargo:

```bash
cargo install --path cyto
```

Or from source:

```bash
git clone github.com:arcinstitute/cyto
cd cyto
cargo install --path crates/cyto
```

## Usage

### Structure

`cyto` is organized into two main components:

1. `cyto map`: Mapping reads to target sequences and generating [IBU files](https://github.com/noamteyssier/ibu)
2. `cyto ibu`: Processing IBU files to sort and count molecules.

The internal subcommands of these components are modular and can be used independently for various stages of the analysis pipeline.

However, `cyto` includes built-in pipelines for common workflows with the `workflow` subcommand.

### Mapping reads to target sequences

`cyto map` is used to map reads to target sequences and generate IBU files.

The general structure of the usage is as follows:

```bash
# Map BINSEQ file
cyto map <mode> -b <BINSEQ> -c <feature_table> -p <probe_file> -o <output_dir>
# Map FASTQ files
cyto map <mode> -i <R1> -I <R2> -c <feature_table> -p <probe_file> -o <output_dir>
```

Where:

1. `<mode>` is the mapping mode (e.g. `crispr`, `gex`, `generic`)
3. `-b` is the path to the BINSEQ file
3. `-i <R1> -I <R2>` are the paths to the R1 and R2 fastq files, respectively
4. `-c <feature_table>` is a feature table that maps the target sequences to their corresponding barcodes. Each mode has a specific format for the feature table.
5. `-p <probe_file>` is an **optional probe file** that can be used to demultiplex the reads by probe sequences.
6. `-o <output_dir>` is the output directory where results will be written (defaults to `./cyto_out`)

The output of `cyto map` creates a structured directory containing IBU files, statistics, and metadata organized as follows:

```text
Output Directory Structure
--------------------------
<output_dir>/
├── metadata/
│   └── features.tsv          # Feature index for the target sequences
├── stats/
│   └── mapping.json         # Statistics on the mapping process
└── ibu/
    ├── output.ibu           # IBU file (single output mode)
    └── <probe_name>.ibu     # IBU files (one per probe when using probe demultiplexing)
```

#### CRISPR processing

The CRISPR mode is used to map reads from CRISPR screens to their corresponding guide sequences.

The expected structure of the feature table is a 3 column TSV file with the following columns:

```text
1. Name of the guide
2. Anchor sequence nucleotides
3. Protospacer sequence nucleotides
```

**Note:** There should be no header in the feature table.

```bash
cyto map crispr \
    -b data/sequencing/sample.bq \
    -c data/libraries/crispr_guides.tsv \
    -o output_directory
```

#### GEX processing

The GEX mode is used to map reads from GEX systems to their corresponding barcodes.

The expected structure of the feature table is a 3 column TSV file with the following columns:

```text
1. Name of the barcode
2. Name of the aggregation (duplicate of barcode if not aggregating genes)
3. Barcode sequence nucleotides
```

**Note:** There should be no header in the feature table.

```bash
cyto map gex \
    -b data/sequencing/sample.bq \
    -c data/libraries/gex_barcodes.tsv \
    -o output_directory
```

#### Probe-based processing

Both of the above methods can be combined with probe-based multiplexing.

The probe file is a 3 column TSV file with the following columns:

```text
1. True sequence of the probe
2. Probe sequence alias
3. Probe name
```

**Note:** There should be no header in the probe file.

**Note:** The nucleotide sequence to match of the probe is not necessarily the same as the probe sequence. These sequences should be provided by 10X Genomics or the manufacturer.

```bash
# Mapping CRISPR - demultiplexing by probe
cyto map crispr \
    -b data/sequencing/sample.bq \
    -c data/libraries/crispr_guides.tsv \
    -p data/metadata/probe-barcodes-fixed-rna-profiling.txt \
    -o output_directory

# Mapping GEX - demultiplexing by probe
cyto map gex \
    -b data/sequencing/sample.bq \
    -c data/libraries/gex_barcodes.tsv \
    -p data/metadata/probe-barcodes-fixed-rna-profiling.txt \
    -o output_directory
```

### Multi-threading

You can take advantage of multi-threading by specifying the number of threads with the `-T` flag.
By default this is set to 8 threads, but it can be set to the number of available cores on your machine with the `-T0` flag.
If you're running on a machine with limited resources, you can set the number of threads to 1 with the `-T1` flag.

### Processing IBU files

Once the reads have been mapped to target sequences and an IBU file has been generated, the `cyto ibu` command can be used to process the IBU file.

Many of these commands make use of multi-threading, see each subcommands `--help` for details.

#### Sorting IBU files

The output of `cyto map` is an unsorted IBU file. To sort the IBU file, use the `sort` subcommand:

```bash
# Sorting an IBU file
cyto ibu sort -i sample.ibu -o sample.sorted.ibu
```

#### Correcting Cellular Barcodes to a Whitelist

A common operation for single-cell sequencing is to correct observed barcodes that are within a certain Hamming distance of a whitelist of known barcodes.
This can be done using the `barcode` subcommand:

```bash
# Correcting cellular barcodes to a whitelist
cyto ibu barcode -i sample.ibu -o sample.corrected.ibu -w data/metadata/737K-fixed-rna-profiling.txt.gz
```

#### Correcting Unique Molecular Identifiers (UMIs)

Another common operation is to correct for low-abundance UMIs that are within a minimal Hamming distance to more abundant UMIs within a Cell-Barcode+Transcript.
This can be done using the `umi` subcommand:

```bash
# Correcting UMIs to a whitelist
cyto ibu umi -i sample.ibu -o sample.corrected.ibu
```

#### Counting molecules

The `count` subcommand is used to generate the barcode-index count matrix after deduplicating UMIs.
This is the count matrix that would be used for downstream single-cell sequencing analyses.
It expects a sorted IBU file as input.

```bash
# Counting molecules from a sorted IBU file
cyto ibu count -i sample.sorted.ibu -o sample.counts.tsv

# Piping the sort and count commands
cyto ibu sort -i sample.ibu -p | cyto ibu count -o sample.counts.tsv

# Including the feature names in the output
# This is useful for downstream analyses
cyto ibu count -i sample.sorted.ibu -o sample.counts.tsv -f output_directory/metadata/features.tsv

# Write counts as mtx (will be written to a subdirectory)
cyto ibu count -i sample.sorted.ibu -f output_directory/metadata/features.tsv -o cyto_out_mtx
```

**Note:** The features are generated from `cyto map` and are located in the `metadata/features.tsv` file within the output directory. These are used to map the feature sequences to their numerical index in the count matrix.

### Workflow Commands

`cyto` includes automated workflow commands that combine mapping and IBU processing steps. These workflows automatically handle the directory structure and process all IBU files (including probe-demultiplexed files) through the complete pipeline:

```bash
# Complete CRISPR workflow: map → sort → barcode correction → UMI correction → count
cyto workflow crispr \
    -b sample.bq \
    -c crispr_guides.tsv \
    -w whitelist.txt \
    -o workflow_output

# Complete GEX workflow: map → sort → barcode correction → UMI correction → count
cyto workflow gex \
    -b sample.bq \
    -c gex_barcodes.tsv \
    -w whitelist.txt \
    -o workflow_output
```

The workflow commands create an extended directory structure:

```text
Workflow Output Directory Structure
-----------------------------------
<output_dir>/
├── metadata/
│   └── features.tsv          # Feature index
├── stats/
│   └── mapping.json         # Mapping statistics
├── ibu/
│   └── *.sort.ibu           # Final processed IBU files
└── counts/
    └── *.counts.tsv         # Count matrices (one per sample/probe)
```

To convert the MTX to an [h5ad](https://anndata.readthedocs.io/en/latest/) file, see attached `scripts/mtx_to_h5ad`.
Dependencies and runtime of script is managed by the python package manager [uv](https://docs.astral.sh/uv/).

```bash
# make script executable
chmod +x ./scripts/mtx_to_h5ad.py

# Convert the mtx directory to h5ad
./scripts/mtx_to_h5ad.py <your_mtx_dir> output.h5ad
```
