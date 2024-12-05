use super::{Index, IndexCounts};

#[derive(Debug, Default)]
pub struct TrackedIndexCounter {
    // The counts for each index
    counts: IndexCounts,
    // The index with the maximum abundance
    pub max_index: Index,
    // The abundance of the maximum index
    pub max_abundance: usize,
}
impl TrackedIndexCounter {
    pub fn increment(&mut self, index: Index) {
        // Pulls the count for the given index
        let count = self.counts.entry(index).or_insert(0);

        // Increments the count
        *count += 1;

        // Updates the tracker
        if *count > self.max_abundance {
            self.max_abundance = *count;
            self.max_index = index;
        }
    }
    pub fn top_index(&self) -> Index {
        self.max_index
    }
}
