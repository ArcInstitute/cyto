use std::io::stdout;

use anyhow::Result;
use scmap::crispr::Library as CrisprLibrary;
use scmap::probe::Library as ProbeLibrary;
use scmap::{BusCounter, BusCounterWriter, PairedReader};

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
    let mut bus_counter = BusCounter::default();

    for pair in PairedReader::new(filepath_r1, filepath_r2)? {
        let bus = pair.as_bus(barcode_size, umi_size);
        let guide_index = guide_mapper.map(&bus.seq, offset);
        let probe = probe_mapper.map(&bus.seq, offset);
        match (guide_index, probe) {
            (Some(g_idx), Some(p_idx)) => {
                // println!("Guide: {g_idx} :: Probe {p_idx}");
                bus_counter.increment(&bus, g_idx);
            }
            _ => {}
        }
    }

    let bus_counter_writer = BusCounterWriter::new(&bus_counter);
    let handle = stdout();
    // bus_counter_writer.write_matrix(&mut handle.lock())?;
    bus_counter_writer.write_sparse(&mut handle.lock())?;

    Ok(())
}
