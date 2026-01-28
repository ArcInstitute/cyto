use anyhow::Result;

use crate::{ILLUMINA_QUALITY_OFFSET, UMI_MIN_QUALITY, v2::geometry::ReadMate};

/// An extraction struct for the UMI of a read
#[derive(Clone, Copy)]
pub struct UmiMapper {
    pos: usize,
    len: usize,
    mate: ReadMate,
}
impl UmiMapper {
    pub fn new(pos: usize, len: usize, mate: ReadMate) -> Self {
        Self { pos, len, mate }
    }

    /// Returns the mate of the read
    #[inline]
    pub fn mate(&self) -> ReadMate {
        self.mate
    }

    /// Determines the linear range of the UMI extraction
    #[inline]
    pub fn range(&self) -> std::ops::Range<usize> {
        self.pos..self.pos + self.len
    }

    /// Extracts the UMI from the given sequence
    #[inline]
    pub fn extract_umi<'a>(&self, seq: &'a [u8]) -> Option<&'a [u8]> {
        seq.get(self.range())
    }

    /// Extracts the UMI from the given sequence as a 2-bit encoded u64 (bitnuc)
    #[inline]
    pub fn extract_2bit_umi(&self, seq: &[u8]) -> Option<Result<u64>> {
        self.extract_umi(seq)
            .map(|umi| bitnuc::as_2bit_lossy(umi).map_err(anyhow::Error::new))
    }

    /// Validates that all quality scores are above the required threshold
    #[inline]
    pub fn passes_quality_threshold(&self, qual: &[u8]) -> bool {
        qual.get(self.range()) // pull the range
            .is_none_or(|sub_qual| {
                // iter over all quality scores and ensure all are above the threshold
                sub_qual
                    .iter()
                    .all(|q| (*q - ILLUMINA_QUALITY_OFFSET) >= UMI_MIN_QUALITY)
            }) // default to true
    }
}
