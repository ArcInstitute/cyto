use std::{fmt::Display, time::Duration};

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct ModuleTiming {
    ibu_name: String,
    module: Module,
    /// Elapsed time in seconds
    elapsed: f64,
}
impl ModuleTiming {
    pub fn new(ibu_name: &str, module: Module, elapsed: Duration) -> Self {
        Self {
            ibu_name: ibu_name.to_string(),
            module,
            elapsed: elapsed.as_secs_f64(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum Module {
    Mapping,
    InitialSort,
    UmiCorrection,
    ReadsDump,
    Counting,
    ConversionH5ad,
    DropletFiltering,
    GuideAssignment,
}
impl Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Module::Mapping => write!(f, "Mapping"),
            Module::InitialSort => write!(f, "InitialSort"),
            Module::UmiCorrection => write!(f, "UmiCorrection"),
            Module::ReadsDump => write!(f, "ReadsDump"),
            Module::Counting => write!(f, "Counting"),
            Module::ConversionH5ad => write!(f, "ConversionH5ad"),
            Module::DropletFiltering => write!(f, "DropletFiltering"),
            Module::GuideAssignment => write!(f, "GuideAssignment"),
        }
    }
}
