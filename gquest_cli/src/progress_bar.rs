use std::cmp::min;

use gquest_core::utils::subject::Observer;
use indicatif::{ProgressBar, ProgressStyle};

/// The progressbar type to display on the terminal, it will also affect how the progress bar reacts to notifications/updates
pub enum ProgressBarType {
    Iterating { start: u64, length: u64 },
    Reading,
}

impl ProgressBarType {
    fn get_style(&self) -> ProgressStyle {
        ProgressStyle::with_template(match self {
            ProgressBarType::Iterating {
                length: _,
                start: _,
            } => "[{spinner:.green} {elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
            ProgressBarType::Reading => "[{spinner:.red} {elapsed_precise}]",
        })
        .expect("Correct style")
        .progress_chars("#|-")
    }
}

pub struct GquestProgressBar {
    pb: ProgressBar,
}

impl Observer for GquestProgressBar {
    fn notify_tick(&mut self) {
        // self.pb.tick(); // Can cause a Bottleneck
    }

    fn notify_data_pushed(&mut self, delta: u64) {
        if let Some(length) = self.pb.length() {
            let new = min(self.pb.position() + delta, length);
            self.pb.set_position(new);

            if length <= self.pb.position() {
                self.pb.finish();
            }
        }
        self.pb.tick();
    }
}

impl GquestProgressBar {
    pub fn new(pb_type: ProgressBarType, message: Option<impl ToString>) -> Self {
        let pb = match &pb_type {
            ProgressBarType::Iterating { start, length } => {
                let pb = ProgressBar::new(*length);
                pb.set_position(*start);
                pb
            }
            ProgressBarType::Reading => ProgressBar::new_spinner(),
        };
        pb.set_style(pb_type.get_style());
        if let Some(msg) = message {
            pb.set_message(msg.to_string());
        }

        Self { pb }
    }

    pub fn force_finish(&self) {
        self.pb.finish();
    }
}
