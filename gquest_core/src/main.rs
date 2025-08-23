use gquest_core::utils::table_handler::QueryTable;
use log::*;

fn main() 
{
    startup_log();
    info!("Program starts");

    // Invariants::execute();
    
    info!("Program ends")
}

/// Starts log environment using the given verbosity arguments
pub fn startup_log()
{
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_target(false)
        .format_timestamp(None)
        .init();
    
}