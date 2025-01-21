use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use anyhow::{bail, Result};
use binseq::{PairedMmapReader, ParallelPairedProcessor, RefRecordPair};
use bitnuc::{decode, split_packed};
use cyto::{
    mappers::{MapperOffset, MappingError},
    statistics::{LibraryCombination, Statistics},
    GeometryR1, Mapper, MappingStatistics,
};

use super::utils::{open_handle, reopen_handle};

#[derive(Debug, Clone)]
pub struct MappingImplementor<M: Mapper> {
    target_mapper: M,
    target_offset: Option<MapperOffset>,
    geometry: GeometryR1,

    local_stats: MappingStatistics,
    global_stats: Arc<Mutex<MappingStatistics>>,

    // Buffers for barcode and UMI sequences used during splitting
    barcode_buf: Vec<u64>,
    umi_buf: Vec<u64>,

    // Decoding buffer for r2 sequence
    dbuf: Vec<u8>,

    // Temporary buffer for output
    local_output_buf: Vec<u8>,

    // Output file name
    filename: String,

    // Lock used to synchronize access to output writer
    output_lock: Arc<Mutex<()>>,

    // Exact matching flag
    exact_matching: bool,
}
impl<M: Mapper> MappingImplementor<M> {
    pub fn new(
        target_mapper: M,
        target_offset: Option<MapperOffset>,
        geometry: GeometryR1,
        filename: String,
        exact_matching: bool,
    ) -> Self {
        let local_stats = MappingStatistics::default();
        let global_stats = Arc::new(Mutex::new(MappingStatistics::default()));
        Self {
            target_mapper,
            target_offset,
            geometry,
            local_stats,
            global_stats,
            barcode_buf: Vec::new(),
            umi_buf: Vec::new(),
            dbuf: Vec::new(),
            local_output_buf: Vec::new(),
            filename,
            output_lock: Arc::new(Mutex::new(())),
            exact_matching,
        }
    }

    fn split_r1(&mut self, pair: &RefRecordPair) -> Result<()> {
        // Split R1 into barcode and UMI
        let r1_config = pair.s_config();
        if r1_config.slen as usize != self.geometry.barcode + self.geometry.umi {
            bail!("R1 sequence length does not match provided geometry");
        }
        split_packed(
            pair.s_seq(),
            r1_config.slen as usize,
            self.geometry.barcode,
            &mut self.barcode_buf,
            &mut self.umi_buf,
        )?;
        if self.barcode_buf.len() != 1 || self.umi_buf.len() != 1 {
            bail!(
                "Barcode split assertion length failed - both barcode and UMI must be under 32bp"
            );
        }
        Ok(())
    }

    fn decode_r2(&mut self, pair: &RefRecordPair) -> Result<()> {
        self.dbuf.clear();
        decode(pair.x_seq, pair.x_config.slen as usize, &mut self.dbuf)?;
        Ok(())
    }

    fn statistics(&self) -> Statistics {
        Statistics::new(
            LibraryCombination::Single(self.target_mapper.library_statistics()),
            self.global_stats
                .lock()
                .expect("Error in aquiring global stats lock")
                .to_owned(),
        )
    }

    fn map_sequence(&self) -> Result<usize, MappingError> {
        if self.exact_matching {
            self.target_mapper
                .map(self.dbuf.as_slice(), self.target_offset)
        } else {
            self.target_mapper
                .map_corrected(self.dbuf.as_slice(), self.target_offset)
        }
    }
}
impl<M: Mapper> ParallelPairedProcessor for MappingImplementor<M> {
    fn process_record_pair(&mut self, pair: binseq::RefRecordPair) -> Result<()> {
        // Split R1 into barcode and UMI
        self.split_r1(&pair)?;
        let barcode = self.barcode_buf[0];
        let umi = self.umi_buf[0];

        // Decode R2
        self.decode_r2(&pair)?;

        // Map the sequence
        match self.map_sequence() {
            Ok(index) => {
                // Write the record
                let record = ibu::Record::new(barcode, umi, index as u64);
                record.write_bytes(&mut self.local_output_buf)?;
                self.local_stats.increment_mapped();
            }
            Err(why) => {
                self.local_stats.increment_unmapped(why);
            }
        }

        Ok(())
    }

    fn on_batch_complete(&mut self) -> Result<()> {
        // Scope the lock to ensure it is released
        {
            let _lock = self
                .output_lock
                .lock()
                .expect("Error in aquiring output lock");
            let mut writer = reopen_handle(&self.filename)?;
            writer.write_all(&self.local_output_buf)?;
            self.local_output_buf.clear();
            writer.flush()?;
        }

        self.global_stats
            .lock()
            .expect("Error in aquiring global stats lock")
            .merge(&self.local_stats);
        self.local_stats.clear();

        Ok(())
    }
}

pub fn ibu_map_pairs_binseq<M>(
    reader: PairedMmapReader,
    filename: String,
    target_mapper: M,
    target_offset: Option<MapperOffset>,
    geometry: GeometryR1,
    num_threads: usize,
    exact_matching: bool,
) -> Result<Statistics>
where
    M: Mapper + 'static,
{
    // Initialize the header and write it to the output file
    {
        let header = ibu::Header::try_from(geometry)?;
        let mut writer = open_handle(&filename)?;
        header.write_bytes(&mut writer)?;
        writer.flush()?;
    }

    // Initialize the mapping implementor
    let implementor = MappingImplementor::new(
        target_mapper,
        target_offset,
        geometry,
        filename,
        exact_matching,
    );

    // Process the records in parallel
    reader.process_parallel(implementor.clone(), num_threads)?;

    // Return the statistics
    Ok(implementor.statistics())
}
