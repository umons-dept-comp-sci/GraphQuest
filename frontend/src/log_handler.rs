use std::{cmp::min, thread::sleep, time::Duration};

use clap_verbosity_flag::*;
use gquest_core::utils::subject::*;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

/// Starts log environment using the given verbosity arguments
pub fn startup_log<T: LogLevel>(verb: Verbosity<T>)
{
    let level = verb.log_level_filter();
    println!("{:?}", level);
    env_logger::builder()
        .filter_level(level)
        .format_target(false)
        .format_timestamp(None)
        .init();
    
}

/// An observer that tracks the progression of the dataset
pub struct DatasetPbObs
{
    geng_progress : ProgressBar,
    is_iterating : bool,
}




impl Observer for DatasetPbObs {
    fn notify(&self, progression: u64) {
        if progression != 0 {
            increase_progress_bar(&self.geng_progress,  {
                if !self.is_iterating {
                    progression
                }else {
                    1
                }
            });
        }else {
            // Ticks the progress bar in order to update the time spent
            self.geng_progress.tick();
        }
    }
}


impl DatasetPbObs {
    
    /// Creates a dataset process by creating a progress bar with a defined style and the given len
    pub fn create() -> Self {
        let pb = ProgressBar::new(0);
        let sty = ProgressStyle::with_template(
            "[{spinner:.green} {elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap()
        .progress_chars("#|-");
        
        
        pb.set_style(sty.clone());
        pb.set_message("TEMP");
        
        Self {
            geng_progress : pb,
            is_iterating : true
        }
    }
    pub fn configure(&mut self, len: u64, message: String, is_iterating: bool)
    {
        self.geng_progress.reset();
        self.is_iterating = is_iterating;
        self.geng_progress.set_length(len);
        self.geng_progress.set_message(message);
    }
}







/// Increase the given progress bar by delta and finishes it when it reaches the end
fn increase_progress_bar(pb: &ProgressBar, delta: u64)
{
    // Increase the progress bar state
    
    let new = min(pb.position() + delta, pb.length().unwrap());
    pb.set_position(new);
    // Check if it is finished
    if pb.length().unwrap() <= pb.position(){
        pb.finish();
    }
}