use gquest_core::{
    data_handler::data_loader::{GengProcess, read_file, read_pipe_signatures},
    database_handler::{GraphDatabase, SqliteGraphDB},
};
use log::{error, info};

use crate::{
    cli_commands::{DatabasePath, DatasetChoice, GengArgs},
    progress_bar::GquestProgressBar,
};

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

    // Create the observer
    let mut pb =
        GquestProgressBar::new(crate::progress_bar::ProgressBarType::Iterating { start: 0, length: 12005168 }, Some("Cool"));

    match input_method {
        DatasetChoice::Geng { args } => {
            let geng_call =
                GengProcess::call_geng(10, &String::new(), (None, None)).expect("correct geng");

            db.add_to_dataset(geng_call.get_reader(), 5000, Some(&mut pb))
                .await;
        }
        DatasetChoice::File { path } => {
            let file = match read_file(&path) {
                Ok(f) => f,
                Err(e) => {
                    error!("Could not open the given file : {e}");
                    return;
                }
            };

            db.add_to_dataset(file, 100, Some(&mut pb)).await;
        }
        DatasetChoice::Pipe => {
            db.add_to_dataset(read_pipe_signatures(), 100, Some(&mut pb))
                .await;
        }
    };
    pb.force_finish();

    info!("Closing database");
    db.close_connection().await;
}
