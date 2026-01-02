use gquest_core::{
    data_handler::data_loader::{GengProcess, read_file, read_pipe_signatures},
    database_handler::SqliteGraphDB,
};
use log::{error, info};

use crate::{
    CliError,
    cli_commands::{BatchSizeArg, DatabasePath, DatasetChoice, RemoveChoice},
    command_handlers::arg_parser::ArgParser,
    progress_bar::GquestProgressBar,
};

/// Creates a database (or simply connects to the existant one) and adds a dataset using the prefered way of the user
pub async fn add_dataset(
    path: DatabasePath,
    input_method: DatasetChoice,
    batch_size: BatchSizeArg,
) -> Result<(), CliError> {
    info!("Creating/connecting database at path : {:?}", path.url);
    let mut db = match SqliteGraphDB::connect_create_graph_database(path.url, None).await {
        Ok(db) => db,
        Err(e) => {
            error!("Error creating/connecting to the given db: {e}");
            return Ok(());
        }
    };
    info!("Importing given dataset");
    let mut pb = GquestProgressBar::new(
        crate::progress_bar::ProgressBarType::Reading,
        Option::<String>::None,
        100,
    );
    // Store the result, close the database, then return the result up
    let val = match input_method {
        DatasetChoice::Geng { args } => {
            // Parse input
            let mut res = Ok(());
            for order in ArgParser::parse_order(&args.order)? {
                pb.set_message(format!("Doing order: {order}"));
                let geng_call = GengProcess::call_geng(
                    order,
                    &args.params.clone().unwrap_or_default(),
                    (None, None),
                )?;

                if let Err(e) = db
                    .add_to_dataset(geng_call.get_reader(), batch_size.batch_size, Some(&mut pb))
                    .await
                {
                    res = Err(e);
                    break;
                }
            }
            pb.set_message("Finished loading dataset");
            res
        }
        DatasetChoice::File { path } => {
            let file = read_file(&path)?;
            pb.set_message(format!("Reading \"{path}\""));
            let res = db
                .add_to_dataset(file, batch_size.batch_size, Some(&mut pb))
                .await;
            pb.set_message(format!("Finished reading \"{path}\""));
            res
        }
        DatasetChoice::Pipe {} => {
            pb.set_message("Reading pipe");
            let res = db
                .add_to_dataset(read_pipe_signatures(), batch_size.batch_size, Some(&mut pb))
                .await;

            pb.set_message("Pipe is closed");
            res
        }
    };
    pb.force_finish();
    info!("Closing database");
    db.close_connection().await;

    Ok(val?)
}

/// Creates a database (or simply connects to the existant one) and adds a dataset using the prefered way of the user
pub async fn remove_dataset(
    path: DatabasePath,
    remove_choice: RemoveChoice,
) -> Result<(), CliError> {
    info!("Opening database");
    let mut db = SqliteGraphDB::connect_graph_database(path.url, None).await?;

    match remove_choice {
        RemoveChoice::Dataset => {
            db.remove_dataset().await?;
        }
        RemoveChoice::All => {
            db.clear_database().await?;
        }
    }
    info!("Closing database");
    db.close_connection().await;
    Ok(())
}
