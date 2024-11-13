use hashbrown::HashMap;

use crate::Bus;

use super::{BarcodeIndexCounter, BusCounter};

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

    pub fn dedup_umi(&self) -> ProbeBarcodeIndexCounter {
        ProbeBarcodeIndexCounter::from_probe_bus_counter(&self)
    }
}

#[derive(Default, Debug)]
pub struct ProbeBarcodeIndexCounter {
    map: HashMap<usize, BarcodeIndexCounter>,
}
impl ProbeBarcodeIndexCounter {
    pub fn from_probe_bus_counter(probe_bus_counter: &ProbeBusCounter) -> Self {
        let mut map = HashMap::new();
        for (p_idx, bus_counter) in &probe_bus_counter.map {
            map.insert(*p_idx, BarcodeIndexCounter::from_counter(bus_counter));
        }
        Self { map }
    }

    pub fn iter_probes(&self) -> impl Iterator<Item = &usize> {
        self.map.keys()
    }

    pub fn get_probe_counter(&self, p_idx: usize) -> Option<&BarcodeIndexCounter> {
        self.map.get(&p_idx)
    }
}
