use hashbrown::HashMap;

use crate::Bus;

use super::BusCounter;

#[derive(Default, Debug)]
pub struct ProbeBusCounter {
    map: HashMap<usize, BusCounter>,
}
impl ProbeBusCounter {
    pub fn increment(&mut self, p_idx: usize, bus: &Bus, g_idx: usize) {
        self.ensure_probe_exists(p_idx);
        self.map.get_mut(&p_idx).unwrap().increment(bus, g_idx);
    }

    fn ensure_probe_exists(&mut self, p_idx: usize) {
        if !self.map.contains_key(&p_idx) {
            self.map.insert(p_idx, BusCounter::default());
        }
    }

    pub fn iter_probes(&self) -> impl Iterator<Item = &usize> {
        self.map.keys()
    }

    pub fn get_probe_counter(&self, p_idx: usize) -> Option<&BusCounter> {
        self.map.get(&p_idx)
    }
}
