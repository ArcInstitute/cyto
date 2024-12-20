use anyhow::{bail, Result};
use bitnuc::{as_2bit, from_2bit};

use crate::aliases::SeqRef;

pub struct Bus<'a> {
    // Barcode encoded as 2-bit using `bitnuc`
    pub barcode: u64,
    // UMI encoded as 2-bit using `bitnuc`
    pub umi: u64,
    // Sequencing read (not encoded)
    pub seq: SeqRef<'a>,
    // Size of the barcode in nucleotides
    pub size_barcode: usize,
    // Size of the UMI in nucleotides
    pub size_umi: usize,
}
impl<'a> Bus<'a> {
    pub fn new(
        r1: SeqRef<'a>,
        r2: SeqRef<'a>,
        size_barcode: usize,
        size_umi: usize,
    ) -> Result<Self> {
        if r1.len() != size_barcode + size_umi {
            bail!("Barcode and UMI sizes do not match the input read size.");
        }
        let barcode = as_2bit(&r1[..size_barcode])?;
        let umi = as_2bit(&r1[size_barcode..])?;
        Ok(Self {
            barcode,
            umi,
            seq: r2,
            size_barcode,
            size_umi,
        })
    }

    pub fn str_barcode<'b>(&self, nuc: &'b mut Vec<u8>) -> Result<&'b str> {
        from_2bit(self.barcode, self.size_barcode, nuc)?;
        Ok(std::str::from_utf8(nuc)?)
    }

    pub fn str_umi<'b>(&self, nuc: &'b mut Vec<u8>) -> Result<&'b str> {
        from_2bit(self.umi, self.size_umi, nuc)?;
        Ok(std::str::from_utf8(nuc)?)
    }
}
