mod correct;
mod count;
mod input;
mod sort;
mod view;

pub use correct::ArgsCorrect;
pub use count::ArgsCount;
pub use input::IbuInput;
pub use sort::ArgsSort;
pub use view::ArgsView;

/// Perform operations on an IBU library
#[derive(clap::Subcommand)]
pub enum IbuCommand {
    /// View the contents of an IBU library as plain text
    View(ArgsView),

    /// Sort the contents of an IBU library
    Sort(ArgsSort),

    /// Create barcode-index count matrix from an IBU library
    Count(ArgsCount),

    /// Correct barcode errors in an IBU library
    ///
    /// Will return a sorted IBU library with corrected barcodes
    Correct(ArgsCorrect),
}
