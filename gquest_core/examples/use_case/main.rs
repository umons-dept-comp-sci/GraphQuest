const DB_SQLITE_URL: &str = "sqlite:gquest_core/examples/use_case/resources/gquest.db";
const CONFIG_PATH: &str = "gquest_core/examples/use_case/resources/configs.json";

use std::time::Duration;

use gquest_core::{
    data_handler::data_loader::GengProcess,
    database_handler::{SqliteGraphDB, SqlxLogLevels},
    parser::query_parser::QueryParser,
    utils::{config_file::ConfigFile, table_handler::QueryTable},
    workplace::Workplace,
};
use log::{LevelFilter, info};

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    startup_log();
    info!("Program starts");

    let log_levels = Some(SqlxLogLevels {
        log_slow_statement_level: Some((LevelFilter::Off, Duration::from_secs(1))),
    });

    let mut db = SqliteGraphDB::connect_create_graph_database(DB_SQLITE_URL, log_levels)
        .await
        .expect("no problem");

    let geng =
        GengProcess::call_geng(None, 8, &"".to_string(), (None, None)).expect("correct call");

    db.add_to_dataset(geng.get_reader(), 1500, None)
        .await
        .expect("correct");

    let config = ConfigFile::read_json_file(&CONFIG_PATH.to_string()).expect("File should correct");

    let mut wp = Workplace::new(db.clone(), config);

    let mut table =
        QueryTable::new_no_header(gquest_core::utils::table_handler::QueryTableOptions::Full);
    wp.find_counterexamples_extremal(
        QueryParser::parse_extr_conj_query("min(ag: n,m), n = 8 => conj1 = 1", None)
            .expect("correct"),
        &mut table,
    )
    .await
    .expect("correct wp");

    // db.clear_database().await.expect("no issues");

    println!("{table}");
    db.close_connection().await;

    info!("Program ends");
}

/// Starts the log environment
pub fn startup_log() {
    env_logger::builder()
        .filter(Some("sqlx::query"), LevelFilter::Debug)
        .filter_level(log::LevelFilter::Debug)
        .format_target(true)
        .format_timestamp(None)
        .init();
}
