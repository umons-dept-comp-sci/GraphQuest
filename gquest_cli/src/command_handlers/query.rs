use gquest_core::{
    database_handler::AllowedGraphDb,
    engine::GquestEngine,
    parser::query_parser::QueryParser,
    utils::{
        StdoutOutput,
        config_file::ConfigFile,
        csv_utils::CsvFile,
        table_handler::{QueryTable, QueryTableOptions},
    },
};
use log::info;

use crate::{
    CliError,
    cli_commands::{ConfigFileArg, DatabasePath, OutputChoice, QueryArgs},
    command_handlers::arg_parser::ArgParser,
};

pub async fn query_raw_sql(path: DatabasePath, query_args: QueryArgs) -> Result<(), CliError> {
    // info!("Opening database");
    // let db = AllowedGraphDb::connect_from_url(path.url, None).await?;

    // info!("Sending raw sql query");

    // let output = get_default_output(query_args.output);

    // match output {
    //     OutputChoice::File { path, separator } => {
    //         // open csv file
    //         let mut csv = CsvFile::new_no_headers(&path, Some(separator))?;
    //         GquestEngine::send_raw_sql(db.clone(), query_args.query, &mut csv).await?;
    //     }
    //     OutputChoice::Stdout => {
    //         GquestEngine::send_raw_sql(db.clone(), query_args.query, &mut StdoutOutput).await?;
    //     }
    //     OutputChoice::Table {
    //         no_id,
    //         partial,
    //         latex,
    //     } => {
    //         let options = if let Some(partial_input) = partial {
    //             ArgParser::parse_partial_table(&partial_input)?
    //         } else {
    //             QueryTableOptions::Full
    //         };
    //         let mut table = QueryTable::new_no_header(!no_id, options);

    //         GquestEngine::send_raw_sql(db.clone(), query_args.query, &mut table).await?;

    //         println!(
    //             "{}",
    //             if latex {
    //                 table.to_latex()
    //             } else {
    //                 table.to_string()
    //             }
    //         );
    //     }
    // };
    // info!("Finished executing query");

    // info!("Closing database");
    // db.close_connection().await;
    // Ok(())
    todo!("add raw sql to core")
}

pub async fn query_database(
    path: DatabasePath,
    query_args: QueryArgs,
    config_arg: ConfigFileArg,
    is_counter: bool,
) -> Result<(), CliError> {
    let output = get_default_output(query_args.output);

    // open database :
    info!("Opening database");
    let db = AllowedGraphDb::connect_from_url(path.url, None).await?;

    info!("Opening configuration file");
    let config = ConfigFile::read_json_file(&config_arg.config_file)?;
    // let formula = QueryParser::change_aliases(query_args.query, config.get_aliases());
    let epsilon = *config.get_epsilon();
    let mut wp = GquestEngine::new(db.clone(), config);

    // Store the result, then close the database even if we encountered an error
    let res = execute_query(&mut wp, output, query_args.query, epsilon, is_counter).await;
    info!("Closing database");
    db.close_connection().await;
    res
}

async fn execute_query(
    engine: &mut GquestEngine,
    output: OutputChoice,
    query: String,
    epsilon: Option<f64>,
    counter: bool,
) -> Result<(), CliError> {
    // try to parse query:
    let query = QueryParser::parse_query(query.clone(), epsilon)?;
    info!("Executing query with the engine");
    match output {
        OutputChoice::File { path, separator } => {
            // open csv file
            let mut csv = CsvFile::new_no_headers(&path, Some(separator))?;
            engine.exec_query(query, &mut csv).await?;
        }
        OutputChoice::Stdout => {
            engine.exec_query(query, &mut StdoutOutput).await?;
        }
        OutputChoice::Table {
            no_id,
            partial,
            latex,
        } => {
            let options = if let Some(partial_input) = partial {
                ArgParser::parse_partial_table(&partial_input)?
            } else {
                QueryTableOptions::Full
            };
            let mut table = QueryTable::new_no_header(!no_id, options);
            engine.exec_query(query, &mut table).await?;
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
    }
    info!("Finished executing query");
    Ok(())
}

pub async fn summary(path: DatabasePath, output: Option<OutputChoice>) -> Result<(), CliError> {
    // open database :
    info!("Opening database");
    let mut db = AllowedGraphDb::connect_from_url(path.url, None).await?;

    let output = get_default_output(output);

    let res = match output {
        OutputChoice::File {
            path: _,
            separator: _,
        } => {
            println!("Not yet implemented");
            Ok(())
        }
        OutputChoice::Stdout => {
            println!("Not yet implemented");
            Ok(())
        }
        OutputChoice::Table {
            no_id,
            partial,
            latex,
        } => {
            let table_opt = if let Some(p) = partial {
                ArgParser::parse_partial_table(&p)?
            } else {
                QueryTableOptions::Full
            };
            match &mut db {
                AllowedGraphDb::Sqlite(sqlite_db) => {
                    sqlite_db.print_all_tables(!no_id, table_opt, latex).await
                }
                AllowedGraphDb::Postgres(pgsql_db) => {
                    pgsql_db.print_all_tables(!no_id, table_opt, latex).await
                }
            }
        }
    };

    info!("Closing database");
    db.close_connection().await;

    Ok(res?)
}

pub fn get_default_output(output: Option<OutputChoice>) -> OutputChoice {
    output.unwrap_or(OutputChoice::Table {
        no_id: false,
        partial: None,
        latex: false,
    })
}
