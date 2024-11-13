use super::Index;
use crate::Bus;

pub trait Counter
where
    Self: Default,
{
    /// Increments the internal counter for the given index for the given bus
    fn increment(&mut self, _bus: &Bus, _index: Index) {
        unimplemented!("increment not implemented for the current Counter")
    }
    /// Increments the internal counter for the given index for the given bus at the given probe
    fn increment_probe(&mut self, _p_idx: Index, _bus: &Bus, _t_idx: Index) {
        unimplemented!("increment_probe not implemented for the current Counter")
    }
}
