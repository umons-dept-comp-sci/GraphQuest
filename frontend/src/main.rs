use std::{path, process::exit};

use clap::Parser;



use gquest_cli::{cli_commands::*, command_handlers::{dataset_handler::*, delete_handler::delete, invariant_cli_handler::compute, query_handler::query, summary_handler::summary}, log_handler::*};
use gquest_core::db_handler::{graph_database::Workspace, sqlite_handler::SqliteGraphDatabase};


#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = CliArg::parse();
    startup_log(args.verbose);

    match args.cmd {
        Modes::Init{path,input_method} => init(path,input_method).await,
        Modes::Add{path,input_method} => add(path,input_method).await,
        Modes::Compute { path, programs, max_processes } => compute(path, programs, max_processes).await, 
        Modes::Query{formula,path, output: output_args } => query(output_args, formula, path).await,
        Modes::Delete{path,table_name} => delete(path, table_name).await,
        Modes::Summary{path} => summary(path).await,
    }
}


