use gquest_core::db_handler::{sqlite_handler::SqliteGraphDatabase, workplace::Workspace};
use log::info;

use crate::{cli_commands::DatabasePath, try_connect_workspace};



pub async fn summary(path: DatabasePath)
{
    // connect to database
    info!("Connecting to the database");
    let wp: Workspace<SqliteGraphDatabase> = try_connect_workspace(path).await;
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