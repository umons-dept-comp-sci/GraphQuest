use std::time::Duration;

use gquest_core::{
    data_handler::data_loader::GengProcess,
    database_handler::{GraphDatabase, SqliteGraphDatabase, SqlxLogLevels},
    utils::config_file::ConfigFile,
};
use log::*;

const DB_URL: &str = "sqlite:resources/gquest.db";
const CONFIG_PATH: &str = "resources/configs.json";

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    startup_log();
    info!("Program starts");

    let log_levels = Some(SqlxLogLevels {
        log_slow_statement_level: Some((LevelFilter::Off, Duration::from_secs(1))),
        log_statement_level: Some(LevelFilter::Off),
    });

    // Create (or connects) to the given database url.
    let mut db =
        GraphDatabase::<SqliteGraphDatabase>::connect_create_graph_database(DB_URL, log_levels)
            .await
            .expect("Database to be okay");

    let _config =
        ConfigFile::read_json_file(&CONFIG_PATH.to_string()).expect("File should correct");

    let geng = GengProcess::call_geng(8, &"".to_string(), (None, None)).expect("correct call");

    db.add_to_dataset(geng.get_reader(), 10000)
        .await
        .expect("correct");

    db.print_all_tables().await.expect("good");

    println!("{:?}", db.get_size_of_table("Dataset").await);

    db.close_connection().await;
    info!("Program ends");
}

/// Starts the log environment
pub fn startup_log() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_target(true)
        .format_timestamp(None)
        .init();
}

/*

let sorter: ExecutableSorter = config.executables.clone().try_into().expect("Good sorter");

let man = sorter.group_execs().expect("good manager");
println!("{man}");

let geng = GengProcess::call_geng(4, &"".to_string(), (None, None)).expect("correct call");

*/
