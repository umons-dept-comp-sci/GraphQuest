use gquest_core::{
    database_handler::{AllowedGraphDb, ClassSelection, ExtremalCounterQuery, SqlCondition},
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
    let db = AllowedGraphDb::connect_from_url(path.url, None).await?;

    info!("Opening configuration file");
    let config = ConfigFile::read_json_file(&query_args.config_file)?;
    let epsilon = *config.get_epsilon();
    let mut wp = Workplace::new(db.clone(), config);

    // Store the result, then close the database even if we encountered an error
    let res = execute_query(&mut wp, output, query_args.query, epsilon).await;
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

    info!("Opening database");
    let db = AllowedGraphDb::connect_from_url(path.url, None).await?;

    info!("Opening configuration file");
    let config = ConfigFile::read_json_file(&query_args.config_file)?;
    let epsilon = *config.get_epsilon();
    let mut wp = Workplace::new(db.clone(), config);

    // Store the result, then close the database even if we encountered an error
    let res = execute_counter(&mut wp, output, query_args.query, epsilon).await;
    info!("Closing database");
    db.close_connection().await;
    res
}

async fn execute_query(
    wp: &mut Workplace,
    output: OutputChoice,
    formula: String,
    epsilon: Option<f64>,
) -> Result<(), CliError> {
    // try to parse query:
    let res = QueryParser::parse_query(formula.clone(), epsilon)?;

    let wrong_command_error = Err(CliError::WrongQueryError {
        formula,
        correct_command: "COUNTER".to_string(),
        current_command: "QUERY".to_string(),
    });

    match res {
        ParsedQuery::Condition(sql_condition) => {
            workplace_condition_query(wp, output, sql_condition).await
        }
        ParsedQuery::Extremal((selection, add_cond)) => {
            workplace_extremal_query(wp, output, selection, add_cond).await
        }
        ParsedQuery::ExtremalCounter(_conj_query) => wrong_command_error,
        ParsedQuery::Counter((_left_cond, _right_cond)) => wrong_command_error,
    }
}

async fn execute_counter(
    wp: &mut Workplace,
    output: OutputChoice,
    formula: String,
    epsilon: Option<f64>,
) -> Result<(), CliError> {
    // try to parse query:
    let res = QueryParser::parse_query(formula.clone(), epsilon)?;

    let wrong_command_error = Err(CliError::WrongQueryError {
        formula,
        correct_command: "QUERY".to_string(),
        current_command: "COUNTER".to_string(),
    });

    match res {
        ParsedQuery::Condition(sql_condition) => {
            workplace_condition_query(wp, output, SqlCondition::not(sql_condition)).await
        }
        ParsedQuery::Extremal((_selection, _add_cond)) => wrong_command_error,
        ParsedQuery::ExtremalCounter(conj_query) => {
            workplace_extremal_counterexample(wp, output, conj_query).await
        }
        ParsedQuery::Counter((left_cond, right_cond)) => {
            workplace_counterexample(wp, output, left_cond, right_cond).await
        }
    }
}

async fn workplace_condition_query(
    wp: &mut Workplace,
    output: OutputChoice,
    cond: SqlCondition,
) -> Result<(), CliError> {
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
    wp: &mut Workplace,
    output: OutputChoice,
    selection: ClassSelection,
    add_cond: Option<SqlCondition>,
) -> Result<(), CliError> {
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
    wp: &mut Workplace,
    output: OutputChoice,
    conj_query: ExtremalCounterQuery,
) -> Result<(), CliError> {
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
    wp: &mut Workplace,
    output: OutputChoice,
    left_cond: SqlCondition,
    right_cond: SqlCondition,
) -> Result<(), CliError> {
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
    let mut db = AllowedGraphDb::connect_from_url(path.url, None).await?;

    let table_opt = if let Some(partial) = partial {
        ArgParser::parse_partial_table(&partial)?
    } else {
        QueryTableOptions::Full
    };
    let res = match &mut db {
        AllowedGraphDb::Sqlite(sqlite_db) => sqlite_db.print_all_tables(table_opt).await,
        AllowedGraphDb::Postgres(pgsql_db) => pgsql_db.print_all_tables(table_opt).await,
    };

    info!("Closing database");
    db.close_connection().await;

    Ok(res?)
}
