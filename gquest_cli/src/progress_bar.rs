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
            ProgressBarType::Reading => "[{spinner:.green}] - {elapsed_precise} - {msg}",
        })
        .expect("Correct style")
        .progress_chars("#|-")
    }
}

pub struct GquestProgressBar {
    pb: ProgressBar,
    update_delay: u64,
    current: u64,
}

impl Observer for GquestProgressBar {
    fn notify_tick(&mut self) {
        // self.pb.tick(); // Can cause a Bottleneck
    }

    fn notify_data_pushed(&mut self, delta: u64) {
        self.current += delta;
        if self.current >= self.update_delay {
            if let Some(length) = self.pb.length() {
                let new = min(self.pb.position() + self.current, length);
                self.pb.set_position(new);

                if length <= self.pb.position() {
                    self.pb.finish();
                }
            }
            self.current = 0;
            self.pb.tick();
        }
    }
}

impl GquestProgressBar {
    pub fn new(
        pb_type: ProgressBarType,
        message: Option<impl ToString>,
        update_delay: u64,
    ) -> Self {
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

        Self {
            pb,
            update_delay,
            current: 0,
        }
    }

    pub fn force_finish(&self) {
        self.pb.finish();
    }

    pub fn set_message(&mut self, new_msg: impl Into<String>) {
        self.pb.set_message(new_msg.into());
    }
}
