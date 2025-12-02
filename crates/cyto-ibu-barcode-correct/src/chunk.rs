use std::fs;
use std::io::{self, BufReader, BufWriter, Take};

use ext_sort::ExternalChunk;
use ibu::{BinaryFormatError, Record};

/// Custom external chunk that uses ibu's native binary format
pub struct IbuExternalChunk {
    reader: io::Take<io::BufReader<fs::File>>,
}

impl ExternalChunk<Record> for IbuExternalChunk {
    type SerializationError = io::Error;
    type DeserializationError = io::Error;

    fn new(reader: Take<BufReader<fs::File>>) -> Self {
        IbuExternalChunk { reader }
    }

    fn dump(
        chunk_writer: &mut BufWriter<fs::File>,
        items: impl IntoIterator<Item = Record>,
    ) -> Result<(), Self::SerializationError> {
        for item in items {
            item.write_bytes(chunk_writer)?;
        }
        Ok(())
    }
}

impl Iterator for IbuExternalChunk {
    type Item = Result<Record, io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.reader.limit() == 0 {
            None
        } else {
            match Record::from_bytes(&mut self.reader) {
                Ok(Some(record)) => Some(Ok(record)),
                Ok(None) => None,
                Err(e) => Some(Err(convert_binary_error(e))),
            }
        }
    }
}

/// Convert BinaryFormatError to io::Error
fn convert_binary_error(err: BinaryFormatError) -> io::Error {
    match err {
        BinaryFormatError::Io(e) => e,
        other => io::Error::new(io::ErrorKind::InvalidData, other),
    }
}
