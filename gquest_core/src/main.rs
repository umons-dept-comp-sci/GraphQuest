use std::time::Duration;

use gquest_core::database_handler::{GraphDatabase, SqliteGraphDatabase, SqlxLogLevels};
use log::*;

const DB_URL: &str = "sqlite:resources/gquest.db";

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    startup_log();
    info!("Program starts");

    let log_levels = Some(SqlxLogLevels {
        log_slow_statement_level: Some((LevelFilter::Off, Duration::from_secs(1))),
        log_statement_level: Some(LevelFilter::Off),
    });

    // This is for mySql, postgre,

    let db =
        GraphDatabase::connect_graph_database(DB_URL, SqliteGraphDatabase::default(), log_levels)
            .await
            .expect("Database to be existant");

    db.close_connection().await;

    info!("Program ends");
}

/// Starts log environment using the given verbosity arguments
pub fn startup_log() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_target(true)
        .format_timestamp(None)
        .init();
}
