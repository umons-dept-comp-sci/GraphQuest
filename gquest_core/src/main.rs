use std::time::Duration;

use gquest_core::{
    data_handler::{data_loader::GengProcess, invariant_execs::InvariantsExecutable},
    database_handler::{GraphDatabase, SqliteGraphDatabase, SqlxLogLevels},
};
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

    let db = GraphDatabase::<SqliteGraphDatabase>::connect_graph_database(DB_URL, log_levels)
        .await
        .expect("Database to be existant");

    let identity = InvariantsExecutable::new(
        "/home/axel/GitProject/GraphQuest/gquest_core/tests/modules/identity.py".to_string(),
        ['x'.to_string()].to_vec(),
        [].to_vec(),
    )
    .expect("correct inv");

    let geng = GengProcess::call_geng(5, &"".to_string(), (None, None)).expect("correct call");

    identity
        .execute_invariant(geng.get_reader(), &mut |_| {
            // println!("s: {}", s);
        })
        .expect("ok");

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
