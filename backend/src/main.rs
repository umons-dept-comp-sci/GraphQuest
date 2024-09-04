use gquest_core::{data_handler::{self, invariant_handlers::{InvariantsExecutable, InvariantsOrderHandler}}, db_handler::{graph_database::*, sqlite_handler::*}, utils::{table_handler::QueryTable, write_csv::{as_line, CsvFile}}};
use log::info;

const THREADS_AVAILABLE: usize = 4;


const DB_URL: &str = "resources/gquest.db";


#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    
}
