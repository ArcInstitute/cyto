

# Metadata
CRISPR_GUIDES := "./data/libraries/crispr_guides.tsv"
GEX_PROBES := "./data/libraries/gex_probes.tsv"
PROBE_BARCODES := "./data/metadata/probe-barcodes-fixed-rna-profiling.txt"

# Input Sequences
GEX_FASTQ_R1 := "./data/sequencing/gex_R1.fastq.gz"
GEX_FASTQ_R2 := "./data/sequencing/gex_R2.fastq.gz"
CRISPR_FASTQ_R1 := "./data/sequencing/crispr_R1.fastq.gz"
CRISPR_FASTQ_R2 := "./data/sequencing/crispr_R2.fastq.gz"

install:
    cargo install --path crates/cyto

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

run-gex-probe: install
    time cyto map gex \
        -i {{GEX_FASTQ_R1}} \
        -I {{GEX_FASTQ_R2}} \
        -c {{GEX_PROBES}} \
        -p {{PROBE_BARCODES}}

run-gex: install
    time cyto map gex \
        -i {{GEX_FASTQ_R1}} \
        -I {{GEX_FASTQ_R2}} \
        -c {{GEX_PROBES}}

clean:
    rm -rfv cyto_out/
