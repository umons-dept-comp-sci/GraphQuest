use gquest_core::{
    database_handler::SqliteGraphDB,
    parser::query_parser::QueryParser,
    utils::{
        StdoutOutput,
        config_file::ConfigFile,
        csv_utils::CsvFile,
        table_handler::{QueryTable, QueryTableOptions},
    },
    workplace::Workplace,
};
use log::info;

use crate::{
    CliError,
    cli_commands::{DatabasePath, OutputChoice},
    command_handlers::arg_parser::ArgParser,
};

pub async fn query_database(
    output: OutputChoice,
    formula: String,
    config_file: String,
    path: DatabasePath,
) -> Result<(), CliError> {
    // open database :
    info!("Parsing query");
    let query = QueryParser::parse_conj_query(formula)?;
    info!("Opening database");
    let db = SqliteGraphDB::connect_graph_database(path.url, None).await?;

    info!("Opening config file");
    let config = ConfigFile::read_json_file(&config_file)?;
    let mut wp = Workplace::new(db, config);
    // TODO: Encapsulate the error here to close the database after catching it

    info!("Executing query with workplace");
    match output {
        OutputChoice::File { path, separator } => {
            // open csv file
            let mut csv = CsvFile::new_no_headers(&path, Some(separator))?;
            wp.find_counterexamples(query, &mut csv).await?;
        }
        OutputChoice::Stdout => {
            wp.find_counterexamples(query, &mut StdoutOutput).await?;
            info!("Finished executing query");
        }
        OutputChoice::Table { partial } => {
            let options = if let Some(partial_input) = partial {
                ArgParser::parse_partial_table(&partial_input)?
            } else {
                QueryTableOptions::Full
            };
            let mut table = QueryTable::new_no_header(options);
            wp.find_counterexamples(query, &mut table).await?;
            info!("Finished executing query");
            println!("{table}");
        }
    }

    info!("Closing database");
    wp.close_workspace().await;
    Ok(())
}

// ../../../target/release/gquest_cli query resources/conj_1/conj_1.json "max(ag: vertices), r >= 2 => conj1 != 0"
