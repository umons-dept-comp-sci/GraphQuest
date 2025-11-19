use clap::Parser;
use clap_verbosity_flag::{LogLevel, Verbosity};
use gquest_cli::{
    cli_commands::{CliArg, Modes},
    command_handlers::init::{add_dataset},
};

/// Starts log environment using the given verbosity arguments
fn startup_log<T: LogLevel>(verb: Verbosity<T>) {
    let level = verb.log_level_filter();

    env_logger::builder()
        .filter_level(level)
        .format_target(false)
        .format_timestamp(None)
        .init();
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = CliArg::parse();
    startup_log(args.verbose);

    match args.cmd {
        Modes::Add { input_method, path } => add_dataset(path, input_method),
        Modes::Compute {
            programs,
            max_processes,
            path,
        } => todo!(),
        Modes::Delete { table_name, path } => todo!(),
        Modes::Query {
            output,
            formula,
            path,
        } => todo!(),
        Modes::Summary { path } => todo!(),
    }
    .await;
}
