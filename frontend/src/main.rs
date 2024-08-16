use clap::Parser;



use gquest_cli::{cli_commands::*, log_handler::*, command_handlers::dataset_handler::*};


#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = CliArg::parse();
    startup_log(args.verbose);

    //test_progress();
    
    match args.cmd {
        Modes::Init { path, input_method } => init(path, input_method).await,
        Modes::Add {path, input_method } => add(path, input_method).await,
        Modes::Compute { path, dependencies_file, programs } => todo!(),
            
        Modes::Delete { path, table_name } => todo!(),
        Modes::Query { hide_output, do_not_save, formula, path } => todo!(),
        Modes::Summary { path } => todo!(),
    }
}

