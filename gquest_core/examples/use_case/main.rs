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

    // Create (or connects) to the given database url.
    let db =
        GraphDatabase::<SqliteGraphDatabase>::connect_create_graph_database(DB_URL, log_levels)
            .await
            .expect("Database to be okay");

    let identity = InvariantsExecutable::new(
        "/home/axel/GitProject/GraphQuest/gquest_core/tests/modules/identity.py".to_string(),
        ['x'.to_string()].to_vec(),
        [].to_vec(),
    )
    .expect("correct inv");

    let geng = GengProcess::call_geng(5, &"".to_string(), (None, None)).expect("correct call");
    let db_clone = db.clone();
    identity
        .execute_invariant(geng.get_reader(), &mut async |s| {
            // println!("s: {}", s);
            println!("HEY I RECEIVED: {s}");
            db_clone.print_all_tables().await.expect("huh ?");
        })
        .await
        .expect("ok");

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
