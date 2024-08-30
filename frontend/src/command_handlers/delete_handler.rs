use gquest_core::db_handler::{graph_database::Workspace, sqlite_handler::SqliteGraphDatabase};
use log::info;

use crate::cli_commands::DatabasePath;


pub async fn delete(path: DatabasePath, table_name: String)
{
    
    let wp : Workspace<SqliteGraphDatabase> = Workspace::connect_workspace(&path.url).await;

    if let Err(e) = wp.delete_table(&table_name).await
    {   
        log::error!("An error occured while trying to delete \"{}\": {}", table_name, e.to_string());
    }
    else {
        info!("Successfully deleted {}", table_name);
    }

    wp.close_workspace().await;
}