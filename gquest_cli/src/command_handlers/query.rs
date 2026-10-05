use gquest_core::{
    database_handler::AllowedGraphDb, engine::GquestEngine, parser::query_parser::QueryParser,
    utils::config_file::ConfigFile,
};
use log::info;

use crate::{
    CliError,
    cli_commands::{ConfigFileArg, DatabasePath, QueryArgs},
    command_handlers::OutputHandler,
};

pub async fn query_raw_sql(path: DatabasePath, query_args: QueryArgs) -> Result<(), CliError> {
    info!("Opening database");
    let db = AllowedGraphDb::connect_from_url(path.url, None).await?;

    info!("Sending raw sql query");

    let mut output = OutputHandler::new(query_args.output)?;
    GquestEngine::send_raw_sql(db.clone(), query_args.query, &mut output).await?;
    println!("{}", output.close());
    info!("Finished executing query");

    info!("Closing database");
    db.close_connection().await;
    Ok(())
}

pub async fn query_database(
    path: DatabasePath,
    query_args: QueryArgs,
    add_args: Option<String>,
    config_arg: ConfigFileArg,
) -> Result<(), CliError> {
    let output = OutputHandler::new(query_args.output)?;

    // open database :
    info!("Opening database");
    let db = AllowedGraphDb::connect_from_url(path.url, None).await?;

    info!("Opening configuration file");
    let config = ConfigFile::read_json_file(&config_arg.config_file)?;
    // let formula = QueryParser::change_aliases(query_args.query, config.get_aliases());
    let epsilon = *config.get_epsilon();
    let mut wp = GquestEngine::new(db.clone(), config);

    // Store the result, then close the database even if we encountered an error
    let res = execute_query(
        &mut wp,
        output,
        query_args.query,
        add_args.unwrap_or_default(),
        epsilon,
        query_args.counter,
        query_args.retain_sigs,
    )
    .await;

    info!("Closing database");
    db.close_connection().await;
    res
}

async fn execute_query(
    engine: &mut GquestEngine,
    mut output: OutputHandler,
    query: String,
    add_args: String,
    epsilon: Option<f64>,
    counter: bool,
    retain_sig: bool,
) -> Result<(), CliError> {
    // try to parse query:
    let mut query = QueryParser::parse_query(query.clone(), epsilon)?;
    let add_expressions = QueryParser::parse_expression_list(add_args)?;
    if counter {
        query.to_counter();
    }
    info!("Executing query with the engine");
    engine
        .exec_query(query, retain_sig, add_expressions, &mut output)
        .await?;
    println!("{}", output.close());
    info!("Finished executing query");
    Ok(())
}

// pub async fn summary(path: DatabasePath, output: Option<OutputChoice>) -> Result<(), CliError> {
//     // open database :
//     info!("Opening database");
//     let mut db = AllowedGraphDb::connect_from_url(path.url, None).await?;

//     let output = get_default_output(output);

//     let res = match output {
//         OutputChoice::File {
//             path: _,
//             separator: _,
//         } => {
//             println!("Not yet implemented");
//             Ok(())
//         }
//         OutputChoice::Stdout => {
//             println!("Not yet implemented");
//             Ok(())
//         }
//         OutputChoice::Table {
//             no_id,
//             partial,
//             latex,
//         } => {
//             let table_opt = if let Some(p) = partial {
//                 ArgParser::parse_partial_table(&p)?
//             } else {
//                 QueryTableOptions::Full
//             };
//             match &mut db {
//                 #[cfg(any(feature = "sqlite", feature = "sqlite-unbundled"))]
//                 AllowedGraphDb::Sqlite(sqlite_db) => {
//                     sqlite_db.print_all_tables(!no_id, table_opt, latex).await
//                 }
//                 #[cfg(feature = "postgres")]
//                 AllowedGraphDb::Postgres(pgsql_db) => {
//                     pgsql_db.print_all_tables(!no_id, table_opt, latex).await
//                 }
//             }
//         }
//     };

//     info!("Closing database");
//     db.close_connection().await;

//     Ok(res?)
// }
