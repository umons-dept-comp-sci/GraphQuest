use std::{cmp::min, thread::sleep, time::Duration};

use clap_verbosity_flag::*;
use gquest_core::utils::subject::*;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};



/// The progressbar type to display on the terminal, it will also affect how the progress bar reacts to notifications/updates
pub enum ProgressBarType
{
    Iterating,
    Download,
    Reading,
}

impl ProgressBarType {
    fn to_string(&self) -> String
    {
        match self {
            ProgressBarType::Download | ProgressBarType::Iterating => "[{spinner:.green} {elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
            ProgressBarType::Reading => "[{spinner:.red} {elapsed_precise}]",
        }.to_string()
    }
}

/// Starts log environment using the given verbosity arguments
pub fn startup_log<T: LogLevel>(verb: Verbosity<T>)
{
    let level = verb.log_level_filter();
    //println!("{:?}", level);
    env_logger::builder()
        .filter_level(level)
        .format_target(false)
        .format_timestamp(None)
        .init();
    
}

/// An observer that tracks the progression of the dataset
pub struct DatasetPbObs
{
    progress_bar : Option<ProgressBar>,
    bar_type: Option<ProgressBarType>,
}




impl Observer for DatasetPbObs {
    
    fn notify_tick(&self) {
        if let Some(pb) = &self.progress_bar {
            pb.tick();  // Simply update without progressing
        }
    }
    
    fn notify_data_pushed(&self, progression: u64) {
        if let Some(pb) = &self.progress_bar
        {
            // If the read data is what is observed, then update the bar
            if let Some(ProgressBarType::Download) = self.bar_type 
            {
                increase_progress_bar(pb, progression);   
            }   // otherwise simply update it
            else {
                // Ticks the progress bar in order to update the time spent
                pb.tick();
            }
        }
    }
    
    fn notify_iteration(&self) {
        if let Some(pb) = &self.progress_bar {
            increase_progress_bar(pb, 1);
        }
    }
}


impl DatasetPbObs {
    
    /// Creates a dataset process by creating a progress bar with a defined style and the given len
    pub fn create() -> Self {

        
        Self {
            progress_bar : None,
            bar_type : None
        }
    }

    /// Starts the progress bar using the given parameters
    pub fn start_progress(&mut self, len: u64, message: String, bar_type: ProgressBarType)
    {
        let pb = ProgressBar::new(len);
        let sty = ProgressStyle::with_template(&bar_type.to_string())
            .unwrap()
            .progress_chars("#|-");
        pb.set_style(sty.clone());
        pb.set_message(message);
        self.progress_bar = Some(pb);
        self.bar_type = Some(bar_type);
    }


    /// Forces the current progress bar to finish even if it still is running
    pub fn force_finish(& self)
    {
        if let Some(pb) = &self.progress_bar {
            pb.finish();
        }
    }
}







/// Increase the given progress bar by delta and finishes it when it reaches the end without going over the maximum position
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