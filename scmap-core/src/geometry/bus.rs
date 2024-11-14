use crate::aliases::SeqRef;

pub struct Bus<'a> {
    pub barcode: SeqRef<'a>,
    pub umi: SeqRef<'a>,
    pub seq: SeqRef<'a>,
}
impl<'a> Bus<'a> {
    pub fn new(r1: SeqRef<'a>, r2: SeqRef<'a>, size_barcode: usize, size_umi: usize) -> Self {
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
