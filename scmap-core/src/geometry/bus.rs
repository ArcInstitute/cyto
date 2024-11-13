use crate::aliases::RefNuc;

pub struct Bus<'a> {
    pub barcode: RefNuc<'a>,
    pub umi: RefNuc<'a>,
    pub seq: RefNuc<'a>,
}
impl<'a> Bus<'a> {
    pub fn new(r1: RefNuc<'a>, r2: RefNuc<'a>, size_barcode: usize, size_umi: usize) -> Self {
        assert_eq!(r1.len(), size_umi + size_barcode);
        let barcode = &r1[..size_barcode];
        let umi = &r1[size_barcode..];
        Self {
            barcode,
            umi,
            seq: r2,
        }
    }
}
