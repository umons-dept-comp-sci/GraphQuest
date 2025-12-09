use std::time::Duration;

use gquest_core::{
    data_handler::data_loader::GengProcess,
    database_handler::{
        ArgType, ClassSelection, ExtremalCounterExampleQuery, SqlComparison, SqlCondition,
        SqlSelectQuery, SqliteGraphDB, SqlxLogLevels,
    },
    utils::config_file::ConfigFile,
    workplace::Workplace,
};
use log::*;
use sqlx::Sqlite;
use tabled::settings::TableOption;

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
    for i in 1..10 {
        println!("Doing class {i}");
        let geng = GengProcess::call_geng(i, &"c".to_string(), (None, None)).expect("correct call");
        db.add_to_dataset(geng.get_reader(), 1500, None)
            .await
            .expect("correct");
    }

    let config = ConfigFile::read_json_file(&CONFIG_PATH_ECCENTRIC.to_string())
        .expect("File should correct");

    let mut wp = Workplace::new(db, config);

    let select = ExtremalCounterExampleQuery {
        selection: ClassSelection::new(
            gquest_core::database_handler::ClassType::Max,
            "eci",
            vec!["vertices", "m"],
        ),
        additional_condition: Some(SqlCondition::and(
            SqlComparison::GreaterEqual(ArgType::column_name("d_nm"), ArgType::value("3")),
            SqlComparison::GreaterEqual(ArgType::column_name("comp"), ArgType::value("0")),
        )),
        conjecture_to_disprove: SqlComparison::Equal(
            ArgType::column_name("comp"),
            ArgType::value("1"),
        )
        .into(),
    };

    // let select = CounterExampleQuery {
    //     selection: Some(ClassSelection::new(
    //         gquest_core::database_handler::ClassType::Min,
    //         "ag",
    //         vec!["r"],
    //     )),
    //     additional_condition: None,
    //     conjecture_to_disprove: SqlComparison::Equal(
    //         ArgType::column_name("conj1"),
    //         ArgType::value("1"),
    //     )
    //     .into(),
    // };

    // let select = ExtremalCounterExampleQuery {
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
    // };

    let res = wp.find_counterexamples(select).await.expect("no error");
    if let Some(count_example) = res {
        println!("{count_example}");
    } else {
        println!("Did not find any counter examples :(");
    }
    // wp.execute_all_executables().await.expect("sdd");

    // wp.execute_all_executables().await.expect("no errors");

    // println!("{}", select.to_sql::<Sqlite>());

    wp.db.print_all_tables().await.expect("good");
    // println!(
    //     "{}",
    //     wp.db
    //         .fetch_all_row_query(
    //             &select,
    //             gquest_core::utils::table_handler::QueryTableOptions::Full,
    //         )
    //         .await
    //         .expect("no error")
    //         .expect("a table")
    // );
    wp.close_workspace().await;
    info!("Program ends");
}

/// Starts the log environment
pub fn startup_log() {
    env_logger::builder()
        .filter(Some("sqlx::query"), LevelFilter::Warn)
        .filter_level(log::LevelFilter::Off)
        .format_target(true)
        .format_timestamp(None)
        .init();
}
