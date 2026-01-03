use gquest_core::{
    database_handler::{ClassSelection, ExtremalCounterQuery, SqlCondition, SqliteGraphDB},
    parser::query_parser::{ParsedQuery, QueryParser},
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
    output: Option<OutputChoice>,
    query: String,
    config_file: String,
    path: DatabasePath,
) -> Result<(), CliError> {
    let output = output.unwrap_or(OutputChoice::Table { partial: None });
    // open database :
    info!("Opening database");
    let mut db = SqliteGraphDB::connect_graph_database(path.url, None).await?;
    // Store the result, then close the database even if we encountered an error
    let res = execute_query(&mut db, output, query, config_file).await;
    info!("Closing database");
    db.close_connection().await;
    res
}

async fn execute_query(
    db: &mut SqliteGraphDB,
    output: OutputChoice,
    formula: String,
    config_file: String,
) -> Result<(), CliError> {
    info!("Opening config file");
    let config = ConfigFile::read_json_file(&config_file)?;

    // try to parse query:
    let res = QueryParser::parse_query(formula.clone())?;

    match res {
        ParsedQuery::Condition(sql_condition) => {
            workplace_condition_query(db, output, sql_condition, config).await
        }
        ParsedQuery::Extremal((selection, add_cond)) => {
            workplace_extremal_query(db, output, selection, add_cond, config).await
        }
        ParsedQuery::ExtremalCounter(conj_query) => {
            workplace_counterexample(db, output, conj_query, config).await
        }
    }
}

async fn workplace_condition_query(
    db: &mut SqliteGraphDB,
    output: OutputChoice,
    cond: SqlCondition,
    config: ConfigFile,
) -> Result<(), CliError> {
    info!("Opening config file");
    let mut wp = Workplace::new(db, config);

    info!("Executing query with workplace");
    match output {
        OutputChoice::File { path, separator } => {
            // open csv file
            let mut csv = CsvFile::new_no_headers(&path, Some(separator))?;
            wp.find_graphs_condition(cond, &mut csv).await?;
        }
        OutputChoice::Stdout => {
            wp.find_graphs_condition(cond, &mut StdoutOutput).await?;
            info!("Finished executing query");
        }
        OutputChoice::Table { partial } => {
            let options = if let Some(partial_input) = partial {
                ArgParser::parse_partial_table(&partial_input)?
            } else {
                QueryTableOptions::Full
            };
            let mut table = QueryTable::new_no_header(options);
            wp.find_graphs_condition(cond, &mut table).await?;
            info!("Finished executing query");
            println!("{table}");
        }
    };

    Ok(())
}

async fn workplace_extremal_query(
    db: &mut SqliteGraphDB,
    output: OutputChoice,
    selection: ClassSelection,
    add_cond: Option<SqlCondition>,
    config: ConfigFile,
) -> Result<(), CliError> {
    info!("Opening config file");
    let mut wp = Workplace::new(db, config);

    info!("Executing query with workplace");
    match output {
        OutputChoice::File { path, separator } => {
            // open csv file
            let mut csv = CsvFile::new_no_headers(&path, Some(separator))?;
            wp.find_extremals_graphs(selection, add_cond, &mut csv)
                .await?;
        }
        OutputChoice::Stdout => {
            wp.find_extremals_graphs(selection, add_cond, &mut StdoutOutput)
                .await?;
            info!("Finished executing query");
        }
        OutputChoice::Table { partial } => {
            let options = if let Some(partial_input) = partial {
                ArgParser::parse_partial_table(&partial_input)?
            } else {
                QueryTableOptions::Full
            };
            let mut table = QueryTable::new_no_header(options);
            wp.find_extremals_graphs(selection, add_cond, &mut table)
                .await?;
            info!("Finished executing query");
            println!("{table}");
        }
    };

    Ok(())
}

async fn workplace_counterexample(
    db: &mut SqliteGraphDB,
    output: OutputChoice,
    conj_query: ExtremalCounterQuery,
    config: ConfigFile,
) -> Result<(), CliError> {
    info!("Opening config file");
    let mut wp = Workplace::new(db, config);

    info!("Executing query with workplace");
    match output {
        OutputChoice::File { path, separator } => {
            // open csv file
            let mut csv = CsvFile::new_no_headers(&path, Some(separator))?;
            wp.find_counterexamples(conj_query, &mut csv).await?;
        }
        OutputChoice::Stdout => {
            wp.find_counterexamples(conj_query, &mut StdoutOutput)
                .await?;
            info!("Finished executing query");
        }
        OutputChoice::Table { partial } => {
            let options = if let Some(partial_input) = partial {
                ArgParser::parse_partial_table(&partial_input)?
            } else {
                QueryTableOptions::Full
            };
            let mut table = QueryTable::new_no_header(options);
            wp.find_counterexamples(conj_query, &mut table).await?;
            info!("Finished executing query");
            println!("{table}");
        }
    };

    Ok(())
}

pub async fn summary(path: DatabasePath, partial: Option<String>) -> Result<(), CliError> {
    // open database :
    info!("Opening database");
    let db = SqliteGraphDB::connect_graph_database(path.url, None).await?;

    let table_opt = if let Some(partial) = partial {
        ArgParser::parse_partial_table(&partial)?
    } else {
        QueryTableOptions::Full
    };

    let res = db.print_all_tables(table_opt).await;
    info!("Closing database");
    db.close_connection().await;

    Ok(res?)
}

// ../../../target/release/gquest_cli query resources/conj_1/conj_1.json "max(ag: vertices), r >= 2 => conj1 != 0"
