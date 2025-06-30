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
2. FLEX barcoding systems
3. Probe-based multiplexing

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
cyto map <mode> -b <BINSEQ> -c <feature_table> -p <probe_file>
# Map FASTQ files
cyto map <mode> -i <R1> -I <R2> -c <feature_table> -p <probe_file>
```

Where:

1. `<mode>` is the mapping mode (e.g. `crispr`, `flex`, `generic`)
3. `-b` is the path to the BINSEQ file
3. `-i <R1> -I <R2>` are the paths to the R1 and R2 fastq files, respectively
4. `-c <feature_table>` is a feature table that maps the target sequences to their corresponding barcodes. Each mode has a specific format for the feature table.
5. `-p <probe_file>` is an **optional probe file** that can be used to demultiplex the reads by probe sequences.

The output of `cyto map` is either a single `IBU` file, or a collection `IBU` files (one per probe) depending on the presence of the probe argument, as well as statistics and a feature index.

```text
Outputs
-------
- <prefix>.ibu: IBU file containing the mapped reads
    ** or **
- <prefix>.<probe>.ibu: IBU file containing the mapped reads for a specific probe

- <prefix>.stats.json: Statistics on the mapping process
- <prefix>.features.tsv: Feature index for the target sequences
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
    -b sample.bq \
    -c crispr_guides.tsv
```

#### FLEX processing

The FLEX mode is used to map reads from FLEX barcoding systems to their corresponding barcodes.

The expected structure of the feature table is a 3 column TSV file with the following columns:

```text
1. Name of the barcode
2. Name of the aggregation (duplicate of barcode if not aggregating genes)
3. Barcode sequence nucleotides
```

**Note:** There should be no header in the feature table.

```bash
cyto map flex \
    -b sample.bq \
    -c flex_barcodes.tsv
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
    -b sample.bq \
    -c crispr_guides.tsv \
    -p probes.tsv

# Mapping FLEX - demultiplexing by probe
cyto map flex \
    -b sample.bq \
    -c flex_barcodes.tsv \
    -p probes.tsv
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
cyto ibu barcode -i sample.ibu -o sample.corrected.ibu -w whitelist.txt
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
cyto ibu count -i sample.sorted.ibu -o sample.counts.tsv -f features.tsv
```

**Note:** The features are generated from `cyto map` and are used to map the feature sequences to their numerical index in the count matrix.
