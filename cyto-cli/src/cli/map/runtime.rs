use clap::Parser;

#[derive(Debug, Clone, Copy, Parser)]
#[clap(next_help_heading = "Runtime Options")]
pub struct RuntimeOptions {
    /// Number of threads to use
    ///
    /// If 0, the number of threads will be set to the maximum number of threads.
    #[clap(short = 'T', long, default_value = "8")]
    pub num_threads: usize,

    /// Output runtime information
    #[clap(short, long)]
    pub verbose: bool,
}
impl RuntimeOptions {
    pub fn num_threads(&self) -> usize {
        if self.num_threads == 0 {
            num_cpus::get()
        } else {
            self.num_threads
        }
    }
}
