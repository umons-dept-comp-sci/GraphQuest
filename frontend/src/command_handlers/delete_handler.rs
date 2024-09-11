use gquest_core::db_handler::{sqlite_handler::SqliteGraphDatabase, workplace::Workspace};
use log::info;

use crate::{cli_commands::DatabasePath, try_connect_workspace};


pub async fn delete(path: DatabasePath, table_name: String)
{
    
    let wp : Workspace<SqliteGraphDatabase> = try_connect_workspace(path).await;

    if let Err(e) = wp.delete_table(&table_name).await
    {   
        log::error!("An error occured while trying to delete \"{}\": {}", table_name, e.to_string());
    }
    else {
        info!("Successfully deleted {}", table_name);
    }

    wp.close_workspace().await;
}