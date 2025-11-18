use clap::Parser;
use clap_verbosity_flag::{LogLevel, Verbosity};
use gquest_cli::cli_commands::CliArg;

fn main() {
    let args = CliArg::parse();

    startup_log(args.verbose);
}

/// Starts log environment using the given verbosity arguments
pub fn startup_log<T: LogLevel>(verb: Verbosity<T>) {
    let level = verb.log_level_filter();

    env_logger::builder()
        .filter_level(level)
        .format_target(false)
        .format_timestamp(None)
        .init();
}
