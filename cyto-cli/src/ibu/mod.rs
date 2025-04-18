mod barcode;
mod count;
mod input;
mod sort;
mod umi;
mod view;

pub use barcode::ArgsBarcode;
pub use count::ArgsCount;
pub use input::IbuInput;
pub use sort::ArgsSort;
pub use umi::ArgsUmi;
pub use view::ArgsView;

/// Perform operations on an IBU library
#[derive(clap::Subcommand, Debug)]
pub enum IbuCommand {
    /// View the contents of an IBU library as plain text
    View(ArgsView),

    /// Sort the contents of an IBU library
    Sort(ArgsSort),

    /// Create barcode-index count matrix from an IBU library
    Count(ArgsCount),

    /// Correct barcode errors in an IBU library
    ///
    /// Does not require a sorted IBU library as input
    Barcode(ArgsBarcode),

    /// Correct UMI errors in an IBU library
    ///
    /// Expects a sorted IBU library as input
    Umi(ArgsUmi),
}
