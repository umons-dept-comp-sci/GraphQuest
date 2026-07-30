const DB_SQLITE_URL: &str = "sqlite:gquest_core/examples/use_case/resources/gquest.db";
const CONFIG_PATH: &str = "gquest_core/examples/use_case/resources/configs.json";

use std::time::Duration;

use gquest_core::{
    data_handler::data_loader::GengProcess,
    database_handler::{SqliteGraphDB, SqlxLogLevels},
    parser::query_parser::QueryParser,
    utils::config_file2::ConfigFile,
    workplace::GquestEngine,
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

    let config = ConfigFile::read_json_file(
        &"gquest_core/examples/use_case/resources/configs_cooler.json".to_string(),
    )
    .expect("valid config file");
    db.add_to_dataset(
        GengProcess::call_geng(None, 8, &"".to_string(), (None, None))
            .expect("correct call")
            .get_reader(),
        config.get_batch_size(),
        None,
    )
    .await
    .expect("no issues while filling the dataset");

    let mut wp = GquestEngine::new(db, config);

    let cond = QueryParser::parse_condition("eci(e_nm(n,m, D(n,m)))) = 1", None)
        .expect("valid condition");
    println!("Read cond: {cond:?}");
    println!("_____________");

    wp.exec_condition_no_multithread(cond).await.expect("no issues");

    wp.close().await;

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
