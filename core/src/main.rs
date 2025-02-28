use gquest_core::{data_handler::{self, invariant_handlers::{InvariantsExecutable, InvariantsOrderHandler}}, db_handler::{graph_database::*, sqlite_handler::*, workplace::Workspace}, utils::{table_handler::QueryTable, write_csv::{as_line, CsvFile}}};
use log::{error, info};

//const THREADS_AVAILABLE: usize = 4;


const DB_URL: &str = "resources/test_graphs.db";


#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() 
{
    // Start log
    startup_log();
    info!("Program starts");

    
    // info!("Creating workspace database at path : {:?}", DB_URL);

    // match Workspace::<SqliteGraphDatabase>::init_workspace(&DB_URL).await 
    // {
    //     Ok(mut wp) => 
    //     {
    //         info!("Workplace/Database succesfully created");
            
    //         info!("Closing workplace/database");
            
    //         wp.close_workspace().await;
    //     },
    //     Err(e) => 
    //     {
    //         error!("An error occured while trying to create the database : {}", e);
    //     },
    // }

    info!("Program closes");
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