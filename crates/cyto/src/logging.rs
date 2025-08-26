use std::io::{self, Write, stderr};
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use cyto_io::open_file_handle;

pub struct MultiWriter {
    stderr: io::Stderr,
    file: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl MultiWriter {
    pub fn new<P: AsRef<Path>>(log_path: P) -> Result<Self> {
        let file = open_file_handle(log_path)?;

        Ok(MultiWriter {
            stderr: stderr(),
            file: Arc::new(Mutex::new(file)),
        })
    }
}

impl Write for MultiWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Write to stderr
        let _ = self.stderr.write(buf)?;

        // Write to file
        let stripped = strip_ansi_escapes::strip(buf);
        if let Ok(mut file) = self.file.lock() {
            let _ = file.write(&stripped)?;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let _ = self.stderr.flush();
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
        Ok(())
    }
}

pub fn setup_workflow_logging<P: AsRef<Path>>(log_path: P) -> Result<()> {
    let multi_writer = MultiWriter::new(log_path)?;
    env_logger::builder()
        .format_timestamp_millis()
        .filter_level(log::LevelFilter::Info)
        .target(env_logger::Target::Pipe(Box::new(multi_writer)))
        .parse_env("CYTO_LOG")
        .write_style(env_logger::WriteStyle::Always)
        .init();
    Ok(())
}

pub fn setup_default_logging() {
    env_logger::builder()
        .format_timestamp_millis()
        .filter_level(log::LevelFilter::Info)
        .parse_env("CYTO_LOG")
        .init();
}
