

# Metadata
CRISPR_GUIDES := "./data/libraries/crispr_guides.tsv"
GEX_PROBES := "./data/libraries/gex_probes.tsv"
PROBE_BARCODES := "./data/metadata/probe-barcodes-fixed-rna-profiling.txt"
BARCODE_LIST := "./data/metadata/737K-fixed-rna-profiling.txt.gz"

# Input Sequences
GEX_BINSEQ := "./data/sequencing/gex.bq"
GEX_FASTQ_R1 := "./data/sequencing/gex_R1.fastq.gz"
GEX_FASTQ_R2 := "./data/sequencing/gex_R2.fastq.gz"
CRISPR_BINSEQ := "./data/sequencing/crispr.bq"
CRISPR_FASTQ_R1 := "./data/sequencing/crispr_R1.fastq.gz"
CRISPR_FASTQ_R2 := "./data/sequencing/crispr_R2.fastq.gz"

install:
    cargo install --path crates/cyto

run-wf-crispr:
    time cyto workflow crispr \
        -c {{CRISPR_GUIDES}} \
        -p {{PROBE_BARCODES}} \
        -w {{BARCODE_LIST}} \
        --force \
        {{CRISPR_BINSEQ}}

run-wf-gex:
    time cyto workflow gex \
        -c {{GEX_PROBES}} \
        -p {{PROBE_BARCODES}} \
        -w {{BARCODE_LIST}} \
        --force \
        {{GEX_BINSEQ}}

run-all: run-crispr-probe-binseq run-crispr-probe-fastq run-crispr-binseq run-crispr-fastq run-gex-probe-binseq run-gex-probe-fastq run-gex-binseq run-gex-fastq

run-crispr-probe-binseq: install
    time cyto map crispr \
        -c {{CRISPR_GUIDES}} \
        -p {{PROBE_BARCODES}} \
        --force \
        {{CRISPR_BINSEQ}}

run-crispr-probe-fastq: install
    time cyto map crispr \
        -c {{CRISPR_GUIDES}} \
        -p {{PROBE_BARCODES}} \
        --force \
        {{CRISPR_FASTQ_R1}} {{CRISPR_FASTQ_R2}}

run-crispr-binseq: install
    time cyto map crispr \
        -c {{CRISPR_GUIDES}} \
        --force \
        {{CRISPR_BINSEQ}}

run-crispr-fastq: install
    time cyto map crispr \
        -c {{CRISPR_GUIDES}} \
        --force \
        {{CRISPR_FASTQ_R1}} {{CRISPR_FASTQ_R2}}

run-gex-probe-binseq: install
    time cyto map gex \
        -c {{GEX_PROBES}} \
        -p {{PROBE_BARCODES}} \
        --force \
        {{GEX_BINSEQ}}

run-gex-probe-fastq: install
    time cyto map gex \
        -c {{GEX_PROBES}} \
        -p {{PROBE_BARCODES}} \
        --force \
        {{GEX_FASTQ_R1}} {{GEX_FASTQ_R2}}

run-gex-binseq: install
    time cyto map gex \
        -c {{GEX_PROBES}} \
        --force \
        {{GEX_BINSEQ}}

run-gex-fastq: install
    time cyto map gex \
        -c {{GEX_PROBES}} \
        --force \
        {{GEX_FASTQ_R1}} {{GEX_FASTQ_R2}}

clean:
    rm -rfv cyto_out/
