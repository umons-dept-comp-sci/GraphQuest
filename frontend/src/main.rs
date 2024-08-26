use clap::Parser;



use gquest_cli::{cli_commands::*, command_handlers::{dataset_handler::*, invariant_cli_handler::compute}, log_handler::*};


#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = CliArg::parse();
    startup_log(args.verbose);

    //test_progress();
    
    match args.cmd {
        Modes::Init{path,input_method}=>init(path,input_method).await,
        Modes::Add{path,input_method}=>add(path,input_method).await,
        Modes::Compute { path, programs, max_processes } => compute(path, programs, max_processes).await, 
        Modes::Delete{path,table_name}=>todo!(),
        Modes::Query{hide_output,do_not_save,formula,path}=>todo!(),
        Modes::Summary{path}=>todo!(),
    }
}

