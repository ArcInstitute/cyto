mod utils;
mod write;

pub use utils::{match_input, match_input_transparent, match_output, match_output_stderr};
pub use write::{open_file_handle, validate_output_directory, write_features, write_statistics};
