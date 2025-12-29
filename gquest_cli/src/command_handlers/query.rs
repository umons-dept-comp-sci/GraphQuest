use gquest_core::{
    database_handler::SqliteGraphDB, parser::query_parser::QueryParser,
    utils::config_file::ConfigFile, workplace::Workplace,
};
use log::info;

use crate::{
    CliError,
    cli_commands::{DatabasePath, OutputQueryArgs},
};

pub async fn query_database(
    output: OutputQueryArgs,
    formula: String,
    config_file: String,
    path: DatabasePath,
) -> Result<(), CliError> {
    println!("{output:?}");

    // open database :
    info!("Opening database");
    let db = SqliteGraphDB::connect_graph_database(path.url, None).await?;

    info!("Opening config file");
    let config = ConfigFile::read_json_file(&config_file)?;

    info!("Parsing query");
    let query = QueryParser::parse_conj_query(formula)?;

    let mut wp = Workplace::new(db, config);

    info!("Executing query with workplace");
    let query = wp.find_counterexamples(query).await?;
    info!("Finished executing query");

    if let Some(table) = query {
        println!("{table}");
    } else {
        println!("No counter example found...");
    }

    info!("closing database");
    wp.close_workspace().await;
    Ok(())
}

// ../../../target/release/gquest_cli query resources/conj_1/conj_1.json "max(ag: vertices), r >= 2 => conj1 != 0"