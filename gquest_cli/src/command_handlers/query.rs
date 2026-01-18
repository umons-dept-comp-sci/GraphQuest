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
    cli_commands::{DatabasePath, OutputChoice, QueryArgs},
    command_handlers::arg_parser::ArgParser,
};

pub async fn query_database(path: DatabasePath, query_args: QueryArgs) -> Result<(), CliError> {
    let output = query_args
        .output
        .unwrap_or(OutputChoice::Table { partial: None });
    // open database :
    info!("Opening database");
    let mut db = SqliteGraphDB::connect_graph_database(path.url, None).await?;
    // Store the result, then close the database even if we encountered an error
    let res = execute_query(&mut db, output, query_args.query, query_args.config_file).await;
    info!("Closing database");
    db.close_connection().await;
    res
}

pub async fn find_counter_database(
    path: DatabasePath,
    query_args: QueryArgs,
) -> Result<(), CliError> {
    let output = query_args
        .output
        .unwrap_or(OutputChoice::Table { partial: None });
    // open database :
    info!("Opening database");
    let mut db = SqliteGraphDB::connect_graph_database(path.url, None).await?;
    // Store the result, then close the database even if we encountered an error
    let res = execute_counter(&mut db, output, query_args.query, query_args.config_file).await;
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
    let res = QueryParser::parse_query(formula.clone(), *config.get_epsilon())?;

    let wrong_command_error = Err(CliError::WrongQueryError {
        formula,
        correct_command: "COUNTER".to_string(),
        current_command: "QUERY".to_string(),
    });

    match res {
        ParsedQuery::Condition(sql_condition) => {
            workplace_condition_query(db, output, sql_condition, config).await
        }
        ParsedQuery::Extremal((selection, add_cond)) => {
            workplace_extremal_query(db, output, selection, add_cond, config).await
        }
        ParsedQuery::ExtremalCounter(_conj_query) => wrong_command_error,
        ParsedQuery::Counter((_left_cond, _right_cond)) => wrong_command_error,
    }
}

async fn execute_counter(
    db: &mut SqliteGraphDB,
    output: OutputChoice,
    formula: String,
    config_file: String,
) -> Result<(), CliError> {
    info!("Opening config file");
    let config = ConfigFile::read_json_file(&config_file)?;

    // try to parse query:
    let res = QueryParser::parse_query(formula.clone(), *config.get_epsilon())?;

    let wrong_command_error = Err(CliError::WrongQueryError {
        formula,
        correct_command: "QUERY".to_string(),
        current_command: "COUNTER".to_string(),
    });

    match res {
        ParsedQuery::Condition(_sql_condition) => wrong_command_error,
        ParsedQuery::Extremal((_selection, _add_cond)) => wrong_command_error,
        ParsedQuery::ExtremalCounter(conj_query) => {
            workplace_extremal_counterexample(db, output, conj_query, config).await
        }
        ParsedQuery::Counter((left_cond, right_cond)) => {
            workplace_counterexample(db, output, left_cond, right_cond, config).await
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

async fn workplace_extremal_counterexample(
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
            wp.find_counterexamples_extremal(conj_query, &mut csv)
                .await?;
        }
        OutputChoice::Stdout => {
            wp.find_counterexamples_extremal(conj_query, &mut StdoutOutput)
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
            wp.find_counterexamples_extremal(conj_query, &mut table)
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
    left_cond: SqlCondition,
    right_cond: SqlCondition,
    config: ConfigFile,
) -> Result<(), CliError> {
    info!("Opening config file");
    let mut wp = Workplace::new(db, config);

    info!("Executing query with workplace");
    match output {
        OutputChoice::File { path, separator } => {
            // open csv file
            let mut csv = CsvFile::new_no_headers(&path, Some(separator))?;
            wp.find_counterexamples(left_cond, right_cond, &mut csv)
                .await?;
        }
        OutputChoice::Stdout => {
            wp.find_counterexamples(left_cond, right_cond, &mut StdoutOutput)
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
            wp.find_counterexamples(left_cond, right_cond, &mut table)
                .await?;
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
