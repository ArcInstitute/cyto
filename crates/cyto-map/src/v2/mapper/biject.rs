use std::{collections::HashMap, hash::Hash};

#[derive(Debug, Clone)]
pub struct Bijection<T> {
    fwd: HashMap<T, usize>,
    rev: HashMap<usize, T>,
}
impl<T: Eq + Hash + Clone> Bijection<T> {
    pub fn new(elements: &[T]) -> Self {
        let mut fwd = HashMap::default();
        let mut rev = HashMap::default();
        for e in elements.into_iter() {
            let map_len = fwd.len();
            fwd.entry(e.clone()).or_insert_with(|| {
                rev.insert(map_len, e.clone());
                map_len
            });
        }
        Self { fwd, rev }
    }

    pub fn len(&self) -> usize {
        self.fwd.len()
    }

    pub fn get_index(&self, element: &T) -> Option<usize> {
        self.fwd.get(element).copied()
    }

    pub fn get_element(&self, index: usize) -> Option<&T> {
        self.rev.get(&index)
    }
}
