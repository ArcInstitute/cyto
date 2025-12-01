use crossbeam_channel::Receiver;

// Channel-based iterator that implements IntoIterator for the external sorter
pub(crate) struct ChannelIterator {
    pub(crate) receiver: Receiver<Result<ibu::Record, ibu::BinaryFormatError>>,
}

impl Iterator for ChannelIterator {
    type Item = Result<ibu::Record, ibu::BinaryFormatError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver.recv().ok()
    }
}
