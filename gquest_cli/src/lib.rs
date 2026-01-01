use gquest_core::{
    data_handler::data_loader::MethodError,
    database_handler::{GraphDbRuntimeError, GraphDbStartupError},
    parser::query_parser::ParsingError,
    utils::{config_file::ConfigFileError, csv_utils::CsvFileError},
    workplace::WorkplaceError,
};
use thiserror::Error;

pub mod cli_commands;
pub mod progress_bar;

pub mod command_handlers;

#[derive(Debug, Error)]
pub enum CliError {
    #[error(
        "Something went wrong while parsing the following arg - \"{arg}\", possibly missing token : \"{missing_tokens:?}\""
    )]
    ArgParseError {
        arg: String,
        column: usize,
        missing_tokens: Vec<String>,
    },
    #[error("Something went wrong when trying to import signatures -> {0}")]
    MethodErrorMethodError(#[from] MethodError),
    #[error("Something went wrong when starting the database -> {0}")]
    GraphDbStartupError(#[from] GraphDbStartupError),
    #[error("{}", ParsingError::pretty_string(.0))]
    QueryParserError(#[from] ParsingError),
    #[error("Something went wrong with the config file -> {0}")]
    ConfigFileError(#[from] ConfigFileError),
    #[error("Something went wrong with the workplace -> {0}")]
    WorplaceError(#[from] WorkplaceError),
    #[error("Something went wrong with the database -> {0}")]
    GraphDbRuntimeError(#[from] GraphDbRuntimeError),

    #[error("Something went wrong with the output file -> {0}")]
    CsvFileError(#[from] CsvFileError),
}
