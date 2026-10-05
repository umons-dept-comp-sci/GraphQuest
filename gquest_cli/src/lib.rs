use gquest_core::{
    data_handler::data_loader::MethodError,
    database_handler::{GraphDbRuntimeError, GraphDbStartupError},
    engine::EngineError,
    parser::query_parser::ParsingError,
    utils::{config_file::ConfigFileError, csv_utils::CsvFileError},
};
use thiserror::Error;

use crate::command_handlers::arg_parser::ArgParseError;

pub mod cli_commands;
pub mod progress_bar;

pub mod command_handlers;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("Something went wrong when trying to import signatures -> {0}")]
    MethodErrorMethodError(#[from] MethodError),
    #[error("Something went wrong when starting the database -> {0}")]
    GraphDbStartupError(#[from] GraphDbStartupError),
    #[error("{}", ParsingError::pretty_string(.0))]
    QueryParserError(#[from] ParsingError),
    #[error("Something went wrong with the config file -> {0}")]
    ConfigFileError(#[from] ConfigFileError),
    #[error("Something went wrong with the engine -> {0}")]
    WorplaceError(#[from] EngineError),
    #[error("Something went wrong with the database -> {0}")]
    GraphDbRuntimeError(#[from] GraphDbRuntimeError),
    #[error("{0}")]
    ArgParseError(#[from] ArgParseError),
    #[error("{0}")]
    CsvFileError(#[from] CsvFileError),
}
