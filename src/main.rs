use anyhow::Result;
use scmap::{Library, PairedReader};

fn main() -> Result<()> {
    let barcode_size = 16;
    let umi_size = 12;

    let filepath_r1 = "./data/sample1_R1.fastq.gz";
    let filepath_r2 = "./data/sample1_R2.fastq.gz";
    let filepath_guides = "./data/crispr_guides.tsv";

    let guide_mapper = Library::from_tsv(filepath_guides.into())?.into_mapper()?;

    for pair in PairedReader::new(filepath_r1, filepath_r2)? {
        let bus = pair.as_bus(barcode_size, umi_size);
        let guide = guide_mapper.map(&bus.seq, 26);
        println!("{:?}", guide);
    }

    Ok(())
}
