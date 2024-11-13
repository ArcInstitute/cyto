use anyhow::Result;
use scmap::crispr::Library as CrisprLibrary;
use scmap::io::write_sparse_mtx;
use scmap::probe::Library as ProbeLibrary;
use scmap::{PairedReader, ProbeBusCounter};
use std::io::BufWriter;

fn main() -> Result<()> {
    let barcode_size = 16;
    let umi_size = 12;
    let offset = 26;
    let write_header = true;

    let filepath_r1 = "./data/sample1_R1.fastq.gz";
    let filepath_r2 = "./data/sample1_R2.fastq.gz";
    let filepath_guides = "./data/crispr_guides.tsv";
    let filepath_probes = "./data/probe-barcodes-fixed-rna-profiling.txt";

    let output_prefix = "./scmap";

    let guide_mapper = CrisprLibrary::from_tsv(filepath_guides.into())?.into_mapper()?;
    let probe_mapper = ProbeLibrary::from_tsv(filepath_probes.into())?.into_mapper()?;
    let mut counter = ProbeBusCounter::default();

    for pair in PairedReader::new(filepath_r1, filepath_r2)? {
        let bus = pair.as_bus(barcode_size, umi_size);
        let guide_index = guide_mapper.map(&bus.seq, offset);
        let probe = probe_mapper.map(&bus.seq, offset);
        match (guide_index, probe) {
            (Some(g_idx), Some(p_idx)) => {
                counter.increment(p_idx, &bus, g_idx);
            }
            _ => {}
        }
    }

    for p_idx in counter.iter_probes() {
        let probe_alias = probe_mapper.get_alias(*p_idx).unwrap().name_str()?;
        let output_path = format!("{output_prefix}.{probe_alias}.mtx");
        let mut output_handle = std::fs::File::create(output_path).map(BufWriter::new)?;
        let bus_counter = counter.get_probe_counter(*p_idx).unwrap();
        write_sparse_mtx(&mut output_handle, bus_counter, write_header)?;
    }

    Ok(())
}
