use gquest_core::{
    database_handler::SqliteGraphDB, parser::query_parser::QueryParser, utils::config_file2::ConfigFile, engine::GquestEngine,
};

const MEMORY_DB_URL: &str = "sqlite::memory:";
pub const CONFIG_FILE: &str = "tests/modules/config_file2.json";

#[tokio::test]
async fn parse_expression_comparison() {
    let test = SqliteGraphDB::connect_graph_database(MEMORY_DB_URL, None)
        .await
        .expect("no issues");
    let config = ConfigFile::read_json_file(&CONFIG_FILE.to_string()).expect("valid config file");
    let wp = GquestEngine::new(test, config);

    let cond =
        QueryParser::parse_condition("P(G, chromatic_nb) * n < 23 + P(G, chromatic_nb) + Q(\"123.12\")", None).expect("valid condition");
    println!("Read cond: {cond:?}");
    println!("_____________");

    let typed_cond = wp.exec_cond_modules_no_multithread(cond);
    println!("res: {typed_cond:?}");
}
