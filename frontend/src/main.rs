use clap::Parser;



use gquest_cli::{cli_commands::*, command_handlers::{dataset_handler::*, invariant_cli_handler::compute, query_handler::query}, log_handler::*};


#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = CliArg::parse();
    startup_log(args.verbose);

    match args.cmd {
        Modes::Init{path,input_method} => init(path,input_method).await,
        Modes::Add{path,input_method} => add(path,input_method).await,
        Modes::Compute { path, programs, max_processes } => compute(path, programs, max_processes).await, 
        Modes::Query{formula,path, output } => query(output, formula, path).await,
        Modes::Delete{path,table_name} => todo!(),
        Modes::Summary{path} => todo!(),
    }
}

