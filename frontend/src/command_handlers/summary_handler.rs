use gquest_core::db_handler::{graph_database::{GraphDatabase, Workspace}, sqlite_handler::{connect_graph_database, SqliteGraphDatabase}};
use log::info;

use crate::cli_commands::DatabasePath;



pub async fn summary(path: DatabasePath)
{
    // connect to database
    info!("Connecting to the database");
    let wp: Workspace<SqliteGraphDatabase> = Workspace::connect_workspace(&path.url).await;
    info!("Successfully connected to the database");
    info!("Fetching summary");
    let table = wp.summary().await;
    info!("Successfully fetched summary");
    info!("Closing workplace");
    // Simply print the summary
    println!("{}", table.as_string());
    // close database
    wp.close_workspace().await;
    info!("Successfully closed workplace");
}