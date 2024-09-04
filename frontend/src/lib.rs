use std::process::exit;

use cli_commands::DatabasePath;
use gquest_core::db_handler::{graph_database::Workspace, sqlite_handler::SqliteGraphDatabase};

pub mod log_handler;
pub mod cli_commands;
pub mod command_handlers {
    pub mod dataset_handler;
    pub mod invariant_cli_handler;
    pub mod query_handler;
    pub mod summary_handler;
    pub mod delete_handler;
}



pub async fn try_connect_workspace<'a>(path_url: DatabasePath) -> Workspace<'a, SqliteGraphDatabase<'a>>
{
    match Workspace::<SqliteGraphDatabase>::connect_workspace(&path_url.url).await
    {
        Ok(wp) => 
        {
            wp
        },
        Err(e) => 
        {
            log::error!("Could not connect to database: {}", e);
            exit(1)
        },
    }
}