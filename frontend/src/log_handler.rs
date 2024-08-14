use std::{thread::sleep, time::Duration};

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
pub struct DatasetProcess
{
    pub geng_progress : ProgressBar,
    //function_to_call : Box<dyn Fn()>
} 

impl Observer for DatasetProcess {
    fn notify(&self, has_progressed: bool) {
        if has_progressed {
            increase_progress_bar(&self.geng_progress, 1);
        }else {
            // Ticks the progress bar in order to update the time spent
            self.geng_progress.tick();
        }
    }
}

impl DatasetProcess {
    pub fn combine(pb: &ProgressBar) -> Self {
        Self {
            geng_progress: pb.clone()
        }
    }
    /// Creates a dataset process by creating a progress bar with a defined style and the given len
    pub fn create(len: u64) -> Self {
        let pb = ProgressBar::new(len);
        let sty = ProgressStyle::with_template(
            "[{spinner:.green} {elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap()
        .progress_chars("#|-");
    
        pb.set_style(sty.clone());
        pb.set_message("todo");
        Self {
            geng_progress : pb,
        }
    }
}


pub fn test_progress()
{

    let m = MultiProgress::new();
    
    let pb = m.add(indicatif::ProgressBar::new(100));
    let sty = ProgressStyle::with_template(
        "[{duration_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
    .unwrap()
    .progress_chars("#|-");

    pb.set_style(sty.clone());
    pb.set_message("todo");

    let pb2 = m.add(indicatif::ProgressBar::new(10));
    pb2.set_style(sty.clone());
    for i in 0..10 {
        sleep(Duration::from_secs_f32(0.2));
        //pb.println(format!("[+] finished #{}", i));
        //pb.println(pb.duration().as_secs().to_string());
        pb2.inc(1);
    }
    pb2.finish_with_message("your turn :)");
    for i in 0..100 {
        sleep(Duration::from_secs_f32(0.3));
        //pb.println(format!("[+] finished #{}", i));
        pb.inc(1);
    }
    pb.finish_with_message("done");
}


/// Increase the given progress bar by delta and finishes it when it reaches the end
fn increase_progress_bar(pb: &ProgressBar, delta: u64)
{
    // Increase the progress bar state
    pb.inc(delta);
    // Check if it is finished
    if pb.length().unwrap() <= pb.position(){
        pb.finish();
    }
}