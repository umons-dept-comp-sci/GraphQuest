use std::time::Duration;

use gquest_core::{
    data_handler::data_loader::GengProcess,
    database_handler::{SqliteGraphDB, SqlxLogLevels},
    utils::config_file::ConfigFile,
    workplace::Workplace,
};
use log::*;

const DB_URL: &str = "sqlite:gquest_core/examples/use_case/resources/gquest.db";
const CONFIG_PATH: &str = "gquest_core/examples/use_case/resources/configs.json";

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    startup_log();
    info!("Program starts");

    let log_levels = Some(SqlxLogLevels {
        log_slow_statement_level: Some((LevelFilter::Off, Duration::from_secs(1))),
    });

    // Create (or connects) to the given database url.
    let mut db = SqliteGraphDB::connect_create_graph_database(DB_URL, log_levels)
        .await
        .expect("Database to be okay");
    let geng = GengProcess::call_geng(7, &"".to_string(), (None, None)).expect("correct call");
    db.add_to_dataset(geng.get_reader(), 10)
        .await
        .expect("correct");

    let config = ConfigFile::read_json_file(&CONFIG_PATH.to_string()).expect("File should correct");
    let mut wp = Workplace::new(db, config);
    wp.execute_all_executables().await.expect("no errors");

    // db.compute_executable(
    //     &InvariantsExecutable::new_no_dep(
    //         "/home/axel/GitProject/GraphQuest/gquest_core/examples/use_case/resources/is_Bmn.py",
    //         vec!["is_Bmn"],
    //     )
    //     .expect("correct_inv"),
    //     10000,
    // )
    // .await
    // .expect("no errors");

    wp.db.print_all_tables().await.expect("good");
    wp.close_workspace().await;
    info!("Program ends");
}

/// Starts the log environment
pub fn startup_log() {
    env_logger::builder()
        .filter(Some("sqlx::query"), LevelFilter::Warn)
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
