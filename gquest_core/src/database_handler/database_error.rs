use thiserror::Error;

use crate::data_handler::{data_types::ValueTypeError, module::{Module, ModuleExecutionError}};


#[derive(Debug, Error)]
/// Represents errors that can happen when trying to conntect to a database.
pub enum GraphDbStartupError {
    #[error("The given url seems to be from an unsupported database system : \"{url}\"")]
    UnknownDatabaseSystem { url: String },
    #[error("The given database is already created \"{database_name}\"")]
    DatabaseAlreadyCreated { database_name: String },
    #[error("The given database was not found: \"{database_name}\"")]
    DatabaseNotFound { database_name: String },
    #[error("Missing priviledges: \"{0}\"")]
    MissingPrivilege(sqlx::Error),
    #[error("Ran into a unknown database error : \"{0}\"")]
    DatabaseError(sqlx::Error),
}

#[derive(Debug, Error)]
/// Represents databases error that can happen while the database is already connected.
pub enum GraphDbRuntimeError {
    #[error(
        "Tried to execute a module when the dataset was not initialised"
    )]
    DatasetNotInitialisedError,
    #[error("The given table name does not exist \"{table_name}\"")]
    TableNotFoundError { table_name: String },
    #[error("A table was already created: \"{0}\"")]
    TableAlreadyCreatedError(sqlx::Error),
    #[error("The following action is forbidden: \"{action}\"")]
    ForbiddenActionError { action: String },
    #[error("An unknown error happened with the following message: \"{0}\"")]
    UnknownError(sqlx::Error),
    #[error("An error happened when trying to execute a query : \"{0}\"")]
    QueryError(sqlx::Error),
    #[error("A violation happened when trying to execute a query : \"{0}\"")]
    ViolationError(sqlx::Error),
    #[error("Too many bind characters found when working on the query : \"{0}\"")]
    QueryCreationError(String),
    #[error(
        "Could not compute the module \"{0:?}\" because the \"{1}\" is not present in the database"
    )]
    InvariantDependencyError(Module, String),
    #[error("Ran into an error while computing an executable : \"{0}\"")]
    InvariantExecutionError(#[from] ModuleExecutionError),
    #[error("The given value is not a valid signature: \"{0}\"")]
    InvalidSignature(String),
    #[error("Error while parsing a returned value : \"{0}\"")]
    ValueTypeError(#[from]ValueTypeError),
}
