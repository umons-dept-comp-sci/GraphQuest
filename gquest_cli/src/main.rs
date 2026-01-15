use clap::Parser;
use clap_verbosity_flag::{LogLevel, Verbosity};
use gquest_cli::{
    cli_commands::{CliArg, Modes},
    command_handlers::{
        add_remove::{add_dataset, remove_dataset},
        query::{find_counter_database, query_database, summary},
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

    let path = args.path;

    match match args.cmd {
        Modes::Add {
            input_method,
            batch_size,
        } => add_dataset(path, input_method, batch_size).await,
        Modes::Query { args } => query_database(path, args).await,
        Modes::Counter { args } => find_counter_database(path, args).await,
        Modes::Remove { choice } => remove_dataset(path, choice).await,
        Modes::Summary { partial } => summary(path, partial).await,
    } {
        Ok(_) => {}
        Err(e) => {
            error!("{e}");
        }
    };
}
