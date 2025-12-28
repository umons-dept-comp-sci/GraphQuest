use thiserror::Error;

pub mod cli_commands;
pub mod progress_bar;

pub mod command_handlers;

#[derive(Debug, Error)]
pub enum CliError {
    #[error(
        "Something went wrong while parsing the following arg : \"{arg}\", possibly missing token : \"{missing_tokens:?}\""
    )]
    ArgParseError {
        arg: String,
        column: usize,
        missing_tokens: Vec<String>,
    },
}
