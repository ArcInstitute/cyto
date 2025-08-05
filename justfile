

# Metadata
CRISPR_GUIDES := "./data/libraries/crispr_guides.tsv"
GEX_PROBES := "./data/libraries/gex_probes.tsv"
PROBE_BARCODES := "./data/metadata/probe-barcodes-fixed-rna-profiling.txt"

# Input Sequences
GEX_BINSEQ := "./data/sequencing/gex.bq"
GEX_FASTQ_R1 := "./data/sequencing/gex_R1.fastq.gz"
GEX_FASTQ_R2 := "./data/sequencing/gex_R2.fastq.gz"
CRISPR_BINSEQ := "./data/sequencing/crispr.bq"
CRISPR_FASTQ_R1 := "./data/sequencing/crispr_R1.fastq.gz"
CRISPR_FASTQ_R2 := "./data/sequencing/crispr_R2.fastq.gz"

install:
    cargo install --path crates/cyto

run-all: run-crispr-probe-binseq run-crispr-probe-fastq run-crispr-binseq run-crispr-fastq run-gex-probe-binseq run-gex-probe-fastq run-gex-binseq run-gex-fastq

run-crispr-probe-binseq: install
    time cyto map crispr \
        -b {{CRISPR_BINSEQ}} \
        -c {{CRISPR_GUIDES}} \
        -p {{PROBE_BARCODES}} \
        --force

run-crispr-probe-fastq: install
    time cyto map crispr \
        -i {{CRISPR_FASTQ_R1}} \
        -I {{CRISPR_FASTQ_R2}} \
        -c {{CRISPR_GUIDES}} \
        -p {{PROBE_BARCODES}} \
        --force

run-crispr-binseq: install
    time cyto map crispr \
        -b {{CRISPR_BINSEQ}} \
        -c {{CRISPR_GUIDES}} \
        --force

run-crispr-fastq: install
    time cyto map crispr \
        -i {{CRISPR_FASTQ_R1}} \
        -I {{CRISPR_FASTQ_R2}} \
        -c {{CRISPR_GUIDES}} \
        --force

run-gex-probe-binseq: install
    time cyto map gex \
        -b {{GEX_BINSEQ}} \
        -c {{GEX_PROBES}} \
        -p {{PROBE_BARCODES}} \
        --force

run-gex-probe-fastq: install
    time cyto map gex \
        -i {{GEX_FASTQ_R1}} \
        -I {{GEX_FASTQ_R2}} \
        -c {{GEX_PROBES}} \
        -p {{PROBE_BARCODES}} \
        --force

run-gex-binseq: install
    time cyto map gex \
        -b {{GEX_BINSEQ}} \
        -c {{GEX_PROBES}} \
        --force

run-gex-fastq: install
    time cyto map gex \
        -i {{GEX_FASTQ_R1}} \
        -I {{GEX_FASTQ_R2}} \
        -c {{GEX_PROBES}} \
        --force

clean:
    rm -rfv cyto_out/
