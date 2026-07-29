use gquest_core::{
    parser::query_parser::QueryParser, utils::config_file2::ConfigFile, workplace::GquestEngine,
};

pub const CONFIG_FILE: &str = "tests/modules/config_file2.json";

#[test]
fn parse_expression_comparison() {
    let config = ConfigFile::read_json_file(&CONFIG_FILE.to_string()).expect("valid config file");

    let wp = GquestEngine::new(config);

    let cond =
        QueryParser::parse_condition("P(G, chromatic_nb) * n < 23 + P(G, chromatic_nb) + Q(\"123.12\")", None).expect("valid condition");
    println!("Read cond: {cond:?}");
    println!("_____________");

    let typed_cond = wp.exec_condition(cond);
    println!("res: {typed_cond:?}");
}
