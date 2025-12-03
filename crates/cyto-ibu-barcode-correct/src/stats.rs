use std::ops::Add;

use serde::Serialize;

#[derive(Debug, Clone, Copy, Default)]
pub struct CorrectStats {
    /// The total number of records
    pub total: u64,
    /// The number of records that matched the whitelist
    pub matched: u64,
    /// The number of records that were corrected
    pub corrected: u64,
    /// The number of records that were corrected via counts
    pub corrected_via_counts: u64,
    /// The number of records with ambiguous corrections
    pub ambiguous: u64,
    /// The number of records that were not corrected
    pub unchanged: u64,
}
impl Add for CorrectStats {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            total: self.total + rhs.total,
            matched: self.matched + rhs.matched,
            corrected: self.corrected + rhs.corrected,
            ambiguous: self.ambiguous + rhs.ambiguous,
            unchanged: self.unchanged + rhs.unchanged,
            corrected_via_counts: self.corrected_via_counts + rhs.corrected_via_counts,
        }
    }
}

#[derive(Serialize)]
pub struct FormattedStats {
    total: u64,
    matched: u64,
    corrected: u64,
    corrected_via_counts: u64,
    ambiguous: u64,
    unchanged: u64,
    frac_matched: f64,
    frac_corrected: f64,
    frac_corrected_via_counts: f64,
    frac_ambiguous: f64,
    frac_unchanged: f64,
}
impl FormattedStats {
    pub fn new(stats: CorrectStats) -> Self {
        Self {
            total: stats.total,
            matched: stats.matched,
            corrected: stats.corrected,
            corrected_via_counts: stats.corrected_via_counts,
            ambiguous: stats.ambiguous,
            unchanged: stats.unchanged,
            frac_matched: stats.matched as f64 / stats.total as f64,
            frac_corrected: stats.corrected as f64 / stats.total as f64,
            frac_corrected_via_counts: stats.corrected_via_counts as f64 / stats.total as f64,
            frac_ambiguous: stats.ambiguous as f64 / stats.total as f64,
            frac_unchanged: stats.unchanged as f64 / stats.total as f64,
        }
    }
}
