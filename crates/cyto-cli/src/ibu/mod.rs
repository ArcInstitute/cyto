mod cat;
mod count;
mod input;
mod reads;
mod sort;
mod umi;
mod view;

pub use cat::ArgsCat;
pub use count::ArgsCount;
pub use input::{IbuInput, MultiIbuInput};
pub use reads::ArgsReads;
pub use sort::ArgsSort;
pub use umi::ArgsUmi;
pub use view::ArgsView;

/// Perform operations on an IBU library
#[derive(clap::Subcommand, Debug)]
pub enum IbuCommand {
    /// View the contents of an IBU library as plain text
    View(ArgsView),

    /// Concatenate the contents of multiple IBU libraries
    Cat(ArgsCat),

    /// Sort the contents of an IBU library
    Sort(ArgsSort),

    /// Create barcode-index count matrix from an IBU library
    Count(ArgsCount),

    /// Correct UMI errors in an IBU library
    ///
    /// Expects a sorted IBU library as input
    Umi(ArgsUmi),

    /// Determine number of reads and UMIs per barcode
    Reads(ArgsReads),
}
