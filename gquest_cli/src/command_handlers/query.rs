use gquest_core::{
    database_handler::{AllowedGraphDb, ClassSelection, ExtremalConjecture, SqlCondition},
    parser::query_parser::{ParsedCondition, ParsedQuery, QueryParser},
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
    cli_commands::{ConfigFileArg, DatabasePath, OutputChoice, QueryArgs},
    command_handlers::arg_parser::ArgParser,
};

pub async fn query_raw_sql(path: DatabasePath, query_args: QueryArgs) -> Result<(), CliError> {
    info!("Opening database");
    let db = AllowedGraphDb::connect_from_url(path.url, None).await?;

    info!("Sending raw sql query");

    let output = query_args.output.unwrap_or(OutputChoice::Table {
        partial: None,
        latex: false,
    });

    match output {
        OutputChoice::File { path, separator } => {
            // open csv file
            let mut csv = CsvFile::new_no_headers(&path, Some(separator))?;
            Workplace::send_raw_sql(db.clone(), query_args.query, &mut csv).await?;
        }
        OutputChoice::Stdout => {
            Workplace::send_raw_sql(db.clone(), query_args.query, &mut StdoutOutput).await?;
        }
        OutputChoice::Table { partial, latex } => {
            let options = if let Some(partial_input) = partial {
                ArgParser::parse_partial_table(&partial_input)?
            } else {
                QueryTableOptions::Full
            };
            let mut table = QueryTable::new_no_header(options);

            Workplace::send_raw_sql(db.clone(), query_args.query, &mut table).await?;

            println!(
                "{}",
                if latex {
                    table.to_latex()
                } else {
                    table.to_string()
                }
            );
        }
    };
    info!("Finished executing query");

    info!("Closing database");
    db.close_connection().await;
    Ok(())
}

pub async fn query_database(
    path: DatabasePath,
    query_args: QueryArgs,
    config_arg: ConfigFileArg,
    is_counter: bool,
) -> Result<(), CliError> {
    let output = query_args.output.unwrap_or(OutputChoice::Table {
        partial: None,
        latex: false,
    });
    // open database :
    info!("Opening database");
    let db = AllowedGraphDb::connect_from_url(path.url, None).await?;

    info!("Opening configuration file");
    let config = ConfigFile::read_json_file(&config_arg.config_file)?;
    let formula = QueryParser::change_aliases(query_args.query, config.get_aliases());
    let epsilon = *config.get_epsilon();
    let mut wp = Workplace::new(db.clone(), config);

    // Store the result, then close the database even if we encountered an error
    let res = execute_query(&mut wp, output, formula, epsilon, is_counter).await;
    info!("Closing database");
    db.close_connection().await;
    res
}

async fn execute_query(
    wp: &mut Workplace,
    output: OutputChoice,
    formula: String,
    epsilon: Option<f64>,
    counter: bool,
) -> Result<(), CliError> {
    // try to parse query:
    let res = QueryParser::parse_query(formula.clone(), epsilon)?;

    match res {
        ParsedQuery::Condition(parsed_condition) => match parsed_condition {
            ParsedCondition::ExtremalCondition(selection, add_condition) => {
                if counter {
                    Err(CliError::WrongQueryError {
                        formula,
                        correct_command: "QUERY".to_string(),
                        current_command: "COUNTER".to_string(),
                    })
                } else {
                    workplace_extremal_query(wp, output, selection, add_condition).await
                }
            }
            ParsedCondition::Condition(mut sql_condition) => {
                if counter {
                    sql_condition = SqlCondition::not(sql_condition.clone())
                }
                workplace_condition_query(wp, output, sql_condition).await
            }
        },
        ParsedQuery::IfElse(parsed_condition, mut right_cond) => match parsed_condition {
            ParsedCondition::ExtremalCondition(class_selection, sql_condition) => {
                let mut conj_query = ExtremalConjecture {
                    selection: class_selection,
                    additional_condition: sql_condition,
                    conjecture: right_cond,
                };
                if counter {
                    conj_query.to_counter_example();
                }
                workplace_extremal_conjecture(wp, output, conj_query).await
            }
            ParsedCondition::Condition(left_cond) => {
                if counter {
                    right_cond = SqlCondition::not(right_cond);
                }
                workplace_conjecture(wp, output, left_cond, right_cond).await
            }
        },
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
            wp.query_condition(cond, &mut csv).await?;
        }
        OutputChoice::Stdout => {
            wp.query_condition(cond, &mut StdoutOutput).await?;
            info!("Finished executing query");
        }
        OutputChoice::Table { partial, latex } => {
            let options = if let Some(partial_input) = partial {
                ArgParser::parse_partial_table(&partial_input)?
            } else {
                QueryTableOptions::Full
            };
            let mut table = QueryTable::new_no_header(options);
            wp.query_condition(cond, &mut table).await?;
            info!("Finished executing query");
            println!(
                "{}",
                if latex {
                    table.to_latex()
                } else {
                    table.to_string()
                }
            );
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
        OutputChoice::Table { partial, latex } => {
            let options = if let Some(partial_input) = partial {
                ArgParser::parse_partial_table(&partial_input)?
            } else {
                QueryTableOptions::Full
            };
            let mut table = QueryTable::new_no_header(options);
            wp.find_extremals_graphs(selection, add_cond, &mut table)
                .await?;
            info!("Finished executing query");
            println!(
                "{}",
                if latex {
                    table.to_latex()
                } else {
                    table.to_string()
                }
            );
        }
    };

    Ok(())
}

async fn workplace_extremal_conjecture(
    wp: &mut Workplace,
    output: OutputChoice,
    conj_query: ExtremalConjecture,
) -> Result<(), CliError> {
    info!("Executing query with workplace");
    match output {
        OutputChoice::File { path, separator } => {
            // open csv file
            let mut csv = CsvFile::new_no_headers(&path, Some(separator))?;
            wp.query_extremal_conjecture(conj_query, &mut csv).await?;
        }
        OutputChoice::Stdout => {
            wp.query_extremal_conjecture(conj_query, &mut StdoutOutput)
                .await?;
            info!("Finished executing query");
        }
        OutputChoice::Table { partial, latex } => {
            let options = if let Some(partial_input) = partial {
                ArgParser::parse_partial_table(&partial_input)?
            } else {
                QueryTableOptions::Full
            };
            let mut table = QueryTable::new_no_header(options);
            wp.query_extremal_conjecture(conj_query, &mut table).await?;
            info!("Finished executing query");
            println!(
                "{}",
                if latex {
                    table.to_latex()
                } else {
                    table.to_string()
                }
            );
        }
    };

    Ok(())
}

async fn workplace_conjecture(
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
            wp.query_conjecture(left_cond, right_cond, &mut csv).await?;
        }
        OutputChoice::Stdout => {
            wp.query_conjecture(left_cond, right_cond, &mut StdoutOutput)
                .await?;
            info!("Finished executing query");
        }
        OutputChoice::Table { partial, latex } => {
            let options = if let Some(partial_input) = partial {
                ArgParser::parse_partial_table(&partial_input)?
            } else {
                QueryTableOptions::Full
            };
            let mut table = QueryTable::new_no_header(options);
            wp.query_conjecture(left_cond, right_cond, &mut table)
                .await?;
            info!("Finished executing query");
            println!(
                "{}",
                if latex {
                    table.to_latex()
                } else {
                    table.to_string()
                }
            );
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
