use std::{cmp::min, thread::sleep, time::Duration};

use clap_verbosity_flag::*;
use gquest_core::utils::subject::*;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};



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

pub struct ExecutableGroupObs 
{
    multi_progress: MultiProgress,
    pb_vec: Vec<DatasetPbObs>
}

impl ExecutableGroupObs {
    pub fn new() -> Self
    {
        let multi_progress = MultiProgress::new();
        let pb_vec: Vec<DatasetPbObs> = vec![];

        Self {
            multi_progress,
            pb_vec
        }
    }

    /// Clears, initialises and hides the progress bars
    pub fn change_settings(&mut self, dataset_length: usize, inv_execs_path: Vec<String>)
    {
        self.multi_progress.clear().unwrap();
        self.pb_vec.clear();
        for name in inv_execs_path {
            let pb = self.multi_progress.add(ProgressBar::new(dataset_length as u64));
            // hide progress bar
            pb.set_draw_target(ProgressDrawTarget::hidden());
            let dataset_obs = DatasetPbObs::new_from_pb(pb, ProgressBarType::Download, name);
            self.pb_vec.push(dataset_obs);
        }
    }

    /// Unhides the progress bars
    pub fn start(&self)
    {
        for pb in &self.pb_vec {
            if let Some(p) = &pb.progress_bar
            {
                p.set_draw_target(ProgressDrawTarget::stdout());
            }
        }
    }
}

impl Observer for ExecutableGroupObs {
    fn notify_tick(&self, index: Option<usize>) {
        if let Some(i) = index {    
            self.pb_vec[i].notify_tick(None);
        }
        else {
            // tick all of them
            for pb in &self.pb_vec {
                pb.notify_tick(None);
            }
        }
    }

    fn notify_data_pushed(&self, progression: u64, index: Option<usize>) {
        if let Some(i) = index {
            
            self.pb_vec[i].notify_data_pushed(progression, None);
            
        }
    }

    fn notify_iteration(&self, index: Option<usize>) {
        if let Some(i) = index {
            self.pb_vec[i].notify_tick(None);
        }
    }
}


/// An observer that tracks the progression of the dataset
pub struct DatasetPbObs
{
    progress_bar : Option<ProgressBar>,
    bar_type: Option<ProgressBarType>,
}




impl Observer for DatasetPbObs {
    
    fn notify_tick(&self, _: Option<usize>) {
        if let Some(pb) = &self.progress_bar {
            pb.tick();  // Simply update without progressing
        }
    }
    
    fn notify_data_pushed(&self, progression: u64, index: Option<usize>) {
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
    
    fn notify_iteration(&self, _: Option<usize>) {
        if let Some(pb) = &self.progress_bar {
            increase_progress_bar(pb, 1);
        }
    }
}


impl DatasetPbObs {
    
    /// Creates a dataset process by creating a progress bar with a defined style and the given len
    pub fn new() -> Self 
    {        
        Self {
            progress_bar : None,
            bar_type : None
        }
    }
    
    fn new_from_pb(pb: ProgressBar, bar_type: ProgressBarType, message: String) -> Self 
    {

        let sty = ProgressStyle::with_template(&bar_type.to_string())
            .unwrap()
            .progress_chars("#|-");
        pb.set_style(sty.clone());
        pb.set_message(message);
        
        Self {
            progress_bar : Some(pb),
            bar_type : Some(bar_type),
        }

    }

    /// Starts the progress bar using the given parameters
    pub fn change_settings(&mut self, len: u64, start_position: Option<u64>, message: String, bar_type: ProgressBarType, is_hidden: bool)
    {
        let pb = ProgressBar::new(len);
        
        if is_hidden {
            pb.set_draw_target(ProgressDrawTarget::hidden());
        }
        let sty = ProgressStyle::with_template(&bar_type.to_string())
            .unwrap()
            .progress_chars("#|-");
        pb.set_style(sty.clone());
        pb.set_message(message);
        if let Some(pos) = start_position {
            pb.set_position(pos);
        }
        self.progress_bar = Some(pb);
        
        self.bar_type = Some(bar_type);
    }

    /// Unhides the bar and resets the elapse time if it was initialised
    pub fn unhide_bar(&self)
    {
        if let Some(pb) = &self.progress_bar {
            pb.reset_elapsed();
            pb.set_draw_target(ProgressDrawTarget::stdout());
        }
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