use std::time::Duration;

use gquest_core::{
    data_handler::data_loader::GengProcess,
    database_handler::{
        ArgType, ClassSelection, GraphConjecture, SqlComparison, SqlCondition,
        SqlSelectQuery, SqliteGraphDB, SqlxLogLevels,
    },
    utils::config_file::ConfigFile,
    workplace::Workplace,
};
use log::*;
use sqlx::Sqlite;

const DB_URL: &str = "sqlite:gquest_core/examples/use_case/resources/gquest.db";
const CONFIG_PATH: &str = "gquest_core/examples/use_case/resources/configs.json";
const CONFIG_PATH_ECCENTRIC: &str = "gquest_core/examples/use_case/resources/zhang_liu_zhou.json";
const CONFIG_PATH_CONJ_1: &str = "gquest_core/examples/use_case/resources/conj_1/conj_1.json";

#[tokio::main(flavor = "multi_thread")]
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
    let geng = GengProcess::call_geng(4, &"".to_string(), (None, None)).expect("correct call");
    db.add_to_dataset(geng.get_reader(), 1500, None)
        .await
        .expect("correct");

    let config =
        ConfigFile::read_json_file(&CONFIG_PATH.to_string()).expect("File should correct");

    let mut wp = Workplace::new(db, config);
    wp.execute_all_executables().await.expect("no errors");

    // let select: SqlSelectQuery = GraphConjecture {
    //     selection: ClassSelection::new(
    //         gquest_core::database_handler::ClassType::Max,
    //         "eci",
    //         vec!["vertices", "m"],
    //     ),
    //     additional_condition: SqlComparison::GreaterEqual(
    //         ArgType::column_name("d_nm"),
    //         ArgType::value("3"),
    //     )
    //     .into(),
    //     conjecture_to_disprove: SqlComparison::Equal(
    //         ArgType::column_name("comp"),
    //         ArgType::value("1"),
    //     )
    //     .into(),
    // }
    // .into();

    // let select: SqlSelectQuery = GraphConjecture {
    //     selection: ClassSelection::new(
    //         gquest_core::database_handler::ClassType::Min,
    //         "P_Gn",
    //         vec!["vertices", "m"],
    //     ),
    //     additional_condition: None,
    //     conjecture_to_disprove: SqlComparison::Equal(
    //         ArgType::column_name("is_Bmn"),
    //         ArgType::value("1"),
    //     )
    //     .into(),
    // }
    // .into();

    let select: SqlSelectQuery = GraphConjecture {
        selection: None,
        additional_condition: None,
        conjecture_to_disprove: SqlComparison::Equal(
            ArgType::column_name("conj1"),
            ArgType::value("1"),
        )
        .into(),
    }
    .into();

    println!("{}", select.to_sql::<Sqlite>());

    // wp.db.print_all_tables().await.expect("good");
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
