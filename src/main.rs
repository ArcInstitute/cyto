use anyhow::Result;
use scmap::crispr::Library as CrisprLibrary;
use scmap::probe::Library as ProbeLibrary;
use scmap::PairedReader;

fn main() -> Result<()> {
    let barcode_size = 16;
    let umi_size = 12;
    let offset = 26;

    let filepath_r1 = "./data/sample1_R1.fastq.gz";
    let filepath_r2 = "./data/sample1_R2.fastq.gz";
    let filepath_guides = "./data/crispr_guides.tsv";
    let filepath_probes = "./data/probe-barcodes-fixed-rna-profiling.txt";

    let guide_mapper = CrisprLibrary::from_tsv(filepath_guides.into())?.into_mapper()?;
    let probe_mapper = ProbeLibrary::from_tsv(filepath_probes.into())?.into_mapper()?;

    for pair in PairedReader::new(filepath_r1, filepath_r2)? {
        let bus = pair.as_bus(barcode_size, umi_size);
        let guide = guide_mapper.map(&bus.seq, offset);
        let probe = probe_mapper.map(&bus.seq, offset);
        match (guide, probe) {
            (Some(guide), Some(probe)) => {
                println!("Guide: {:?}", guide);
                println!("Probe: {:?}", probe);
            }
            _ => {}
        }
    }

    Ok(())
}
