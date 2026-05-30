//! Simple progress reporting wrapper around `indicatif`.

use indicatif::{ProgressBar, ProgressStyle};

/// Thin wrapper around an `indicatif` progress bar.
pub struct ProgressReporter {
    bar: ProgressBar,
}

impl ProgressReporter {
    pub fn new(total: u64, msg: &str) -> Self {
        let bar = ProgressBar::new(total);
        bar.set_style(
            ProgressStyle::with_template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        bar.set_message(msg.to_string());
        Self { bar }
    }

    pub fn inc(&self, delta: u64) {
        self.bar.inc(delta);
    }

    pub fn finish(&self) {
        self.bar.finish_with_message("done");
    }
}
