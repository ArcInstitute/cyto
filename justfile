

# Metadata
CRISPR_GUIDES := "./data/crispr_guides.tsv"
FLEX_PROBES := "./data/flex_probes.tsv"
PROBE_BARCODES := "./data/probe-barcodes-fixed-rna-profiling.txt"

# Input Sequences
FLEX_FASTQ_R1 := "./data/flex_R1.fastq.gz"
FLEX_FASTQ_R2 := "./data/flex_R2.fastq.gz"
CRISPR_FASTQ_R1 := "./data/crispr_R1.fastq.gz"
CRISPR_FASTQ_R2 := "./data/crispr_R2.fastq.gz"


install-dev:
    cargo install --path cyto-cli -F benchmarking

install:
    cargo install --path cyto-cli

run-crispr-probe: install
    time cyto map crispr \
        -i {{CRISPR_FASTQ_R1}} \
        -I {{CRISPR_FASTQ_R2}} \
        -c {{CRISPR_GUIDES}} \
        -p {{PROBE_BARCODES}}

run-crispr: install
    time cyto map crispr \
        -i {{CRISPR_FASTQ_R1}} \
        -I {{CRISPR_FASTQ_R2}} \
        -c {{CRISPR_GUIDES}}

run-flex-probe: install
    time cyto map flex \
        -i {{FLEX_FASTQ_R1}} \
        -I {{FLEX_FASTQ_R2}} \
        -c {{FLEX_PROBES}} \
        -p {{PROBE_BARCODES}}

run-flex: install
    time cyto map flex \
        -i {{FLEX_FASTQ_R1}} \
        -I {{FLEX_FASTQ_R2}} \
        -c {{FLEX_PROBES}}

clean:
    rm -v cyto_out.*
