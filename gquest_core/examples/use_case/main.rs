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

    let geng = GengProcess::call_geng(8, &"".to_string(), (None, None)).expect("correct call");

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

// async fn example_eccentric(log_levels: Option<SqlxLogLevels>) -> QueryTable {
//     let mut db = SqliteGraphDB::connect_create_graph_database(DB_URL, log_levels)
//         .await
//         .expect("Database to be okay");
//     for i in 1..8 {
//         println!("Doing class {i}");
//         let geng = GengProcess::call_geng(i, &"c".to_string(), (None, None)).expect("correct call");
//         db.add_to_dataset(geng.get_reader(), 1500, None)
//             .await
//             .expect("correct");
//     }

//     let config = ConfigFile::read_json_file(&CONFIG_PATH_ECCENTRIC.to_string())
//         .expect("File should correct");

//     let mut wp = Workplace::new(db, config);

//     let conjecture = ExtremalCounterQuery {
//         selection: ClassSelection::new(
//             gquest_core::database_handler::ClassType::Max,
//             "eci",
//             vec!["vertices", "m"],
//         ),
//         additional_condition: Some(SqlCondition::and(
//             SqlComparison::GreaterEqual(ArgType::identifier("d_nm"), ArgType::value("3")),
//             SqlComparison::GreaterEqual(ArgType::identifier("comp"), ArgType::value("0")),
//         )),
//         conjecture_to_disprove: SqlComparison::Equal(
//             ArgType::identifier("comp"),
//             ArgType::value("1"),
//         )
//         .into(),
//     };
//     let mut res = QueryTable::new_no_header(QueryTableOptions::Full);
//     wp.find_counterexamples(conjecture.clone(), &mut res)
//         .await
//         .expect("no error");
//     println!("{}", conjecture.as_sql::<Sqlite>());
//     wp.close_workspace().await;

//     res
// }

// async fn example_Bmn(log_levels: Option<SqlxLogLevels>) -> QueryTable {
//     let mut db = SqliteGraphDB::connect_create_graph_database(DB_URL, log_levels)
//         .await
//         .expect("Database to be okay");
//     for i in 1..7 {
//         println!("Doing class {i}");
//         let geng = GengProcess::call_geng(i, &"".to_string(), (None, None)).expect("correct call");
//         db.add_to_dataset(geng.get_reader(), 1500, None)
//             .await
//             .expect("correct");
//     }

//     let config = ConfigFile::read_json_file(&CONFIG_PATH.to_string()).expect("File should correct");

//     let mut wp = Workplace::new(db, config);

//     let conjecture = ExtremalCounterQuery {
//         selection: ClassSelection::new(ClassType::Min, "P_Gn", vec!["vertices", "m"]),
//         additional_condition: None,
//         conjecture_to_disprove: SqlComparison::Equal(
//             ArgType::identifier("is_Bmn"),
//             ArgType::value("1"),
//         )
//         .into(),
//     };

//     let mut res = QueryTable::new_no_header(QueryTableOptions::Full);
//     wp.find_counterexamples(conjecture.clone(), &mut res)
//         .await
//         .expect("no error");
//     println!("{}", conjecture.as_sql::<Sqlite>());
//     wp.close_workspace().await;

//     res
// }

// async fn example_conj1(log_levels: Option<SqlxLogLevels>) -> QueryTable {
//     let mut db = SqliteGraphDB::connect_create_graph_database(DB_URL, log_levels)
//         .await
//         .expect("Database to be okay");
//     for i in 1..9 {
//         println!("Doing class {i}");
//         let geng = GengProcess::call_geng(i, &"".to_string(), (None, None)).expect("correct call");
//         db.add_to_dataset(geng.get_reader(), 1500, None)
//             .await
//             .expect("correct");
//     }

//     let config =
//         ConfigFile::read_json_file(&CONFIG_PATH_CONJ_1.to_string()).expect("File should correct");

//     let mut wp = Workplace::new(db, config);

//     let conjecture = ExtremalCounterQuery {
//         selection: ClassSelection::new(ClassType::Min, "ag", vec!["vertices", "r"]),
//         additional_condition: None,
//         conjecture_to_disprove: SqlComparison::Equal(
//             ArgType::identifier("conj1"),
//             ArgType::value("1"),
//         )
//         .into(),
//     };

//     let mut res = QueryTable::new_no_header(QueryTableOptions::Full);
//     wp.find_counterexamples(conjecture.clone(), &mut res)
//         .await
//         .expect("no error");
//     println!("{}", conjecture.as_sql::<Sqlite>());
//     wp.close_workspace().await;

//     res
// }

// async fn example_conj2(log_levels: Option<SqlxLogLevels>) -> QueryTable {
//     let mut db = SqliteGraphDB::connect_create_graph_database(DB_URL, log_levels)
//         .await
//         .expect("Database to be okay");
//     for i in 1..7 {
//         println!("Doing class {i}");
//         let geng = GengProcess::call_geng(i, &"".to_string(), (None, None)).expect("correct call");
//         db.add_to_dataset(geng.get_reader(), 1500, None)
//             .await
//             .expect("correct");
//     }

//     let config =
//         ConfigFile::read_json_file(&CONFIG_PATH_CONJ_2.to_string()).expect("File should correct");

//     let mut wp = Workplace::new(db, config);

//     let conjecture = ExtremalCounterQuery {
//         selection: ClassSelection::new(ClassType::Min, "P_Gn", vec!["vertices", "m"]),
//         additional_condition: None,
//         conjecture_to_disprove: SqlComparison::LessEqual(
//             ArgType::identifier("P_Bmn"),
//             ArgType::identifier("P_Gn"),
//         )
//         .into(),
//     };

//     let mut res = QueryTable::new_no_header(QueryTableOptions::Full);
//     wp.find_counterexamples(conjecture.clone(), &mut res)
//         .await
//         .expect("no error");
//     println!("{}", conjecture.as_sql::<Sqlite>());
//     wp.close_workspace().await;

//     res
// }

/// Starts the log environment
pub fn startup_log() {
    env_logger::builder()
        .filter(Some("sqlx::query"), LevelFilter::Debug)
        .filter_level(log::LevelFilter::Debug)
        .format_target(true)
        .format_timestamp(None)
        .init();
}
