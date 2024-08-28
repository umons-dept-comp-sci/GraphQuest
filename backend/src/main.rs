use gquest_core::{data_handler::{self, invariant_handlers::{InvariantsExecutable, InvariantsOrderHandler}}, db_handler::{graph_database::*, sqlite_handler::*}};

const THREADS_AVAILABLE: usize = 4;


const DB_URL: &str = "resources/gquest.db";


#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    
    let wp : Workspace<SqliteGraphDatabase> = Workspace::connect_workspace(DB_URL).await;
    
    wp.execute_query(&String::from("SELECT * FROM a1 WHERE value = 0"), Some(';'), Some("res.csv".to_string()), OutputOptions::None).await;
    
    wp.close_workspace().await;
}
