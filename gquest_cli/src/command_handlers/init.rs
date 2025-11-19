use gquest_core::{data_handler::data_loader::{read_file, read_pipe_signatures}, database_handler::{GraphDatabase, SqliteGraphDB}};
use log::{error, info};

use crate::cli_commands::{DatabasePath, DatasetChoice, GengArgs};

/// Creates a database (or simply connects to the existant one) and adds a dataset using the prefered way of the user
pub async fn add_dataset(path: DatabasePath, input_method: DatasetChoice) {
    info!("Creating/connecting database at path : {:?}", path.url);
    let mut db = match SqliteGraphDB::connect_create_graph_database(path.url, None).await {
        Ok(db) => db,
        Err(e) => {
            error!("Error creating/connecting to the given db: {e}");
            return;
        }
    };
    info!("Importing given dataset");

    // db.add_to_dataset(match input_method {
    //     DatasetChoice::Geng { args } => todo!(),
    //     DatasetChoice::File { path } => {
    //         match read_file(&path) {
    //             Ok(f) => f,
    //             Err(e) => {
    //                 error!("Could not open the given file : {e}");
    //                 return;
    //             },
    //         }
    //     },
    //     DatasetChoice::Pipe => {
    //         read_pipe_signatures()
    //     }
    // }, 10).await;

    info!("Closing database");
}
