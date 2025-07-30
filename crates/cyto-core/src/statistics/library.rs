use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct LibraryStatistics {
    inner: LibraryCombination,
}
impl LibraryStatistics {
    pub fn new(inner: LibraryCombination) -> Self {
        Self { inner }
    }
}

#[derive(Debug, Serialize, Clone)]
pub enum LibraryCombination {
    Single(Library),
    Dual(Library, Library),
}

#[derive(Debug, Serialize, Clone)]
pub enum Library {
    Probe(ProbeLibraryStatistics),
    Crispr(CrisprLibraryStatistics),
    Gex(GexLibraryStatistics),
    Generic(GenericLibraryStatistics),
}

#[derive(Debug, Default, Serialize, Clone, Copy)]
pub struct ProbeLibraryStatistics {
    pub num_probes: usize,
    pub num_aliases: usize,
    pub probe_size: usize,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct CrisprLibraryStatistics {
    // Anchor Statistics
    pub num_anchors: usize,
    pub anchor_sizes: Vec<usize>,

    // Protospacer Statistics
    pub num_protospacers: usize,
    pub protospacer_size: usize,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct GexLibraryStatistics {
    pub num_gex_sequences: usize,
    pub gex_sequence_size: usize,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct GenericLibraryStatistics {
    pub num_target_sequences: usize,
    pub target_sequence_size: usize,
}
