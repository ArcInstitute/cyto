mod input;
mod view;

pub use input::IbuInput;
pub use view::ArgsView;

/// Perform operations on an IBU library
#[derive(clap::Subcommand)]
pub enum IbuCommand {
    /// View the contents of an IBU library as plain text
    View(ArgsView),
}
