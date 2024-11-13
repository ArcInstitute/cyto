use spinoff::{spinners, Color, Spinner, Streams};
use std::time::Instant;

pub struct ProgressBar {
    spinner: Spinner,
    start: Instant,
    num_events: usize,
    update_frequency: usize,
}
impl Default for ProgressBar {
    fn default() -> Self {
        Self::new(1000000)
    }
}
impl ProgressBar {
    fn build_spinner() -> Spinner {
        Spinner::new_with_stream(
            spinners::Dots,
            "Processing reads...",
            Color::Blue,
            Streams::Stderr,
        )
    }

    pub fn new(update_frequency: usize) -> Self {
        let spinner = Self::build_spinner();
        let start = Instant::now();
        let num_events = 0;
        Self {
            spinner,
            start,
            num_events,
            update_frequency,
        }
    }

    pub fn tick(&mut self) {
        self.num_events += 1;
        if self.num_events % self.update_frequency == 0 {
            self.update_message();
        }
    }

    fn update_message(&mut self) {
        self.spinner.update_text(format!(
            "Processing reads... {:.3}M processed, {:.3} seconds",
            self.num_events as f64 / 1_000_000.0,
            self.start.elapsed().as_secs_f64(),
        ));
    }

    pub fn finish(&mut self) {
        self.spinner.stop_with_message(&format!(
            "Finished processing reads. {:.3}M processed, {:.3} seconds",
            self.num_events as f64 / 1_000_000.0,
            self.start.elapsed().as_secs_f64(),
        ));
    }
}
