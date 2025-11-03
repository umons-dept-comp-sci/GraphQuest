use log::error;
use thiserror::Error;

#[derive(Debug, Error)]
/// Represents errors that can happen when trying to conntect to a database.
pub enum GraphDbStartupError {
    #[error("The given database is already created \"{database_name}\"")]
    DatabaseAlreadyCreated { database_name: String },
    #[error("The given database was not found: \"{database_name}\"")]
    DatabaseNotFound { database_name: String },
    #[error("Ran into a database error : \"{0}\"")]
    DatabaseError(#[from] sqlx::Error),
}

#[derive(Debug, Error)]
/// Represents databases error that can happen while the database is already connected.
pub enum GraphDbRuntimeError {
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
    QueryCreationError(String)
}

impl From<sqlx::Error> for GraphDbRuntimeError {
    fn from(val: sqlx::Error) -> Self {
        sqlx_error_to_db_error(val)
    }
}

fn sqlx_error_to_db_error(val: sqlx::Error) -> GraphDbRuntimeError {
    match &val {
        sqlx::Error::Database(database_error) => match database_error.kind() {
            sqlx::error::ErrorKind::UniqueViolation
            | sqlx::error::ErrorKind::ForeignKeyViolation
            | sqlx::error::ErrorKind::NotNullViolation
            | sqlx::error::ErrorKind::CheckViolation => GraphDbRuntimeError::ViolationError(val),
            sqlx::error::ErrorKind::Other => {
                let e_str = val.to_string();
                if e_str.contains("table") && e_str.contains("already exists") {
                    GraphDbRuntimeError::TableAlreadyCreatedError(val)
                } else if e_str.contains("syntax") {
                    GraphDbRuntimeError::QueryError(val)
                } else {
                    val.into()
                }
            }
            _ => unreachable!("This is not suppose to be reached"),
        },
        _ => {
            // error!("Sqlx error not handled : {}", val);
            val.into()
        }
    }
}
