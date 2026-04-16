use clap::Parser;
use clap_verbosity_flag::{LogLevel, Verbosity};
use gquest_cli::{
    cli_commands::{CliArg, Modes},
    command_handlers::{
        add_remove::{add_dataset, remove_dataset},
        query::{query_database, query_raw_sql, summary},
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
        Modes::Query { args, config } => query_database(path, args, config, false).await,
        Modes::Counter { args, config } => query_database(path, args, config, true).await,
        Modes::Sql { args } => query_raw_sql(path, args).await,
        Modes::Remove { choice } => remove_dataset(path, choice).await,
        Modes::Summary { output } => summary(path, output).await,
    } {
        Ok(_) => {}
        Err(e) => {
            error!("{e}");
        }
    };
}
