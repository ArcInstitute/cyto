use std::io::Write;
use std::sync::Arc;

use anyhow::Result;
use cyto_cli::ArgsView;
use cyto_io::match_output;
use paraseq::parallel::{PairedParallelProcessor, PairedParallelReader};
use parking_lot::Mutex;

type BoxedWriter = Box<dyn Write + Send>;
const DEFAULT_BUFFER_CAPACITY: usize = 1024;

#[derive(Clone)]
pub struct CytoView {
    /// Barcode geometry
    bc_size: usize,
    /// UMI geometry
    umi_size: usize,

    /// Local buffer for storing processed data
    local_buffer: Vec<u8>,

    /// Global output writer
    global_output: Arc<Mutex<BoxedWriter>>,
}
impl CytoView {
    pub fn new(bc_size: usize, umi_size: usize, global_output: Arc<Mutex<BoxedWriter>>) -> Self {
        Self {
            bc_size,
            umi_size,
            local_buffer: Vec::with_capacity(DEFAULT_BUFFER_CAPACITY),
            global_output,
        }
    }
    /// Write the processed data to the local buffer
    fn write_to_local(&mut self, bc: &[u8], umi: &[u8], seq: &[u8]) -> Result<()> {
        self.local_buffer.extend_from_slice(bc);
        self.local_buffer.write_all(b"\t")?;
        self.local_buffer.extend_from_slice(umi);
        self.local_buffer.write_all(b"\t")?;
        self.local_buffer.extend_from_slice(seq);
        self.local_buffer.write_all(b"\n")?;
        Ok(())
    }
}

impl PairedParallelProcessor for CytoView {
    fn process_record_pair<Rf: paraseq::Record>(
        &mut self,
        record1: Rf,
        record2: Rf,
    ) -> paraseq::parallel::Result<()> {
        let r1_seq = record1.seq();

        if r1_seq.len() < (self.bc_size + self.umi_size) {
            return Err(anyhow::anyhow!("Record 1 sequence is too short").into());
        }

        self.write_to_local(
            &r1_seq[..self.bc_size],
            &r1_seq[self.bc_size..self.bc_size + self.umi_size],
            &record2.seq(),
        )?;

        Ok(())
    }

    fn on_batch_complete(&mut self) -> paraseq::parallel::Result<()> {
        {
            let mut global_output = self.global_output.lock();
            global_output.write_all(&self.local_buffer)?;
            global_output.flush()?;
        }
        self.local_buffer.clear();
        Ok(())
    }
}

pub fn run(args: &ArgsView) -> Result<()> {
    // Open readers
    let (r1_reader, r2_reader) = args.input.to_readers()?;

    // Determine number of threads
    let num_threads = args.options.threads.max(1);

    // Open output file
    let writer = match_output(args.options.output.as_ref())?;

    // Initialize processor
    let processor = CytoView::new(
        args.geometry.barcode,
        args.geometry.umi,
        Arc::new(Mutex::new(writer)),
    );

    // Process records in parallel
    r1_reader.process_parallel_paired(r2_reader, processor, num_threads)?;

    Ok(())
}
