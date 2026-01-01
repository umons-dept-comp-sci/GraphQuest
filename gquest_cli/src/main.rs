use clap::Parser;
use clap_verbosity_flag::{LogLevel, Verbosity};
use gquest_cli::{
    cli_commands::{CliArg, Modes},
    command_handlers::{
        init::{add_dataset, remove_dataset},
        query::{query_database, summary},
    },
};
use log::error;

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

    match match args.cmd {
        Modes::Add { input_method, path } => add_dataset(path, input_method).await,
        Modes::Query {
            output,
            formula,
            config_file,
            path,
        } => query_database(output, formula, config_file, path).await,
        Modes::Remove { path, choice } => remove_dataset(path, choice).await,
        Modes::Summary { path, partial } => summary(path, partial).await,
    } {
        Ok(_) => {}
        Err(e) => {
            error!("{e}");
        }
    };
}
