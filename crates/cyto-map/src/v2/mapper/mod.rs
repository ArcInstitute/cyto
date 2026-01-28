mod biject;
mod crispr;
mod gex;
mod probe;
mod umi;
mod whitelist;

pub use biject::Bijection;
pub use crispr::CrisprMapper;
pub use gex::GexMapper;
pub use probe::ProbeMapper;
pub use umi::UmiMapper;
pub use whitelist::WhitelistMapper;

use crate::v2::geometry::ReadMate;

pub trait Mapper {
    /// Queries the mapper for the parent index of the given sequence.
    fn query(&self, seq: &[u8]) -> Option<usize>;

    /// Returns which read (R1/R2) this mapper operates on.
    fn mate(&self) -> ReadMate;
}

impl<T: Mapper + ?Sized> Mapper for Box<T> {
    fn query(&self, seq: &[u8]) -> Option<usize> {
        (**self).query(seq)
    }
    fn mate(&self) -> ReadMate {
        (**self).mate()
    }
}

// Typestate markers
pub struct Unpositioned;
pub struct Ready;
