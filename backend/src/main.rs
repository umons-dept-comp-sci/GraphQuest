use gquest_core::{data_handler::{self, invariant_handlers::{InvariantsExecutable, InvariantsOrderHandler}}, db_handler::{graph_database::*, sqlite_handler::*}};

const THREADS_AVAILABLE: usize = 4;


const DB_URL: &str = "resources/gquest.db";


#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    
    let wp : Workspace<SqliteGraphDatabase> = Workspace::connect_workspace(DB_URL).await;
    
    let inv = InvariantsOrderHandler::read_json(&String::from("resources/invariant_modules/dep.json"));
    
    let top = inv.get_topological_order();


    
    wp.close_workspace().await;
}
