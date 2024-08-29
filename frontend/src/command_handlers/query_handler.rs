use gquest_core::db_handler::{graph_database::*, sqlite_handler::*};
use log::info;
use std::path::Path;
use crate::log_handler::*;
use crate::cli_commands::*;


pub async fn query(output: OutputQueryArgs, formula : String, path : DatabasePath)
{
    let wp: Workspace<SqliteGraphDatabase> = Workspace::connect_workspace(&path.url).await;

    println!("{:?}, {:?}, {:?}", output, formula, path); 
    wp.close_workspace().await;
}
