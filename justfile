# Metadata

CRISPR_GUIDES := "./data/libraries/crispr_guides.tsv"
GEX_PROBES := "./data/libraries/gex_probes.tsv"
PROBE_BARCODES := "./data/metadata/probe-barcodes-fixed-rna-profiling.txt"
BARCODE_LIST := "./data/metadata/737K-fixed-rna-profiling.txt.gz"

# Input Sequences

GEX_BINSEQ := "./data/sequencing/gex.cbq"
GEX_FASTQ_R1 := "./data/sequencing/gex_R1.fastq.gz"
GEX_FASTQ_R2 := "./data/sequencing/gex_R2.fastq.gz"
CRISPR_BINSEQ := "./data/sequencing/crispr.cbq"
CRISPR_FASTQ_R1 := "./data/sequencing/crispr_R1.fastq.gz"
CRISPR_FASTQ_R2 := "./data/sequencing/crispr_R2.fastq.gz"

# Geometries

CRISPR_UNPROBED_GEOMETRY := "[barcode][umi:12] | [:26][anchor][protospacer]"

install:
    export RUSTFLAGS="-C target-cpu=native"; cargo install --path crates/cyto

install-portable:
    cargo install --path crates/cyto

run-wf-crispr:
    time cyto workflow crispr \
        -c {{ CRISPR_GUIDES }} \
        -p {{ PROBE_BARCODES }} \
        -w {{ BARCODE_LIST }} \
        --preset crispr-proper \
        --force \
        {{ CRISPR_BINSEQ }}

run-wf-gex:
    time cyto workflow gex \
        -c {{ GEX_PROBES }} \
        -p {{ PROBE_BARCODES }} \
        -w {{ BARCODE_LIST }} \
        --preset gex-v1 \
        --force \
        {{ GEX_BINSEQ }}

run-all: run-crispr-binseq run-crispr-fastq run-gex-binseq run-gex-fastq

run-crispr-binseq: install
    time cyto map crispr \
        -c {{ CRISPR_GUIDES }} \
        -w {{ BARCODE_LIST }} \
        -p {{ PROBE_BARCODES }} \
        --preset crispr-proper \
        --force \
        {{ CRISPR_BINSEQ }}

run-crispr-binseq-unprobed: install
    time cyto map crispr \
        -c {{ CRISPR_GUIDES }} \
        -w {{ BARCODE_LIST }} \
        --geometry "{{ CRISPR_UNPROBED_GEOMETRY }}" \
        --force \
        {{ CRISPR_BINSEQ }}

run-crispr-fastq: install
    time cyto map crispr \
        -c {{ CRISPR_GUIDES }} \
        -w {{ BARCODE_LIST }} \
        -p {{ PROBE_BARCODES }} \
        --preset crispr-proper \
        --force \
        {{ CRISPR_FASTQ_R1 }} {{ CRISPR_FASTQ_R2 }}

run-gex-binseq: install
    time cyto map gex \
        -c {{ GEX_PROBES }} \
        -p {{ PROBE_BARCODES }} \
        -w {{ BARCODE_LIST }} \
        --force \
        {{ GEX_BINSEQ }}

run-gex-binseq-unprobed: install
    time cyto map gex \
        -c {{ GEX_PROBES }} \
        -w {{ BARCODE_LIST }} \
        --force \
        {{ GEX_BINSEQ }}

run-gex-fastq: install
    time cyto map gex \
        -c {{ GEX_PROBES }} \
        -p {{ PROBE_BARCODES }} \
        -w {{ BARCODE_LIST }} \
        --force \
        {{ GEX_FASTQ_R1 }} {{ GEX_FASTQ_R2 }}

clean:
    rm -rfv cyto_out/
