use log::error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphDbStartupError {
    #[error("The given database is already created \"{database_name}\"")]
    DatabaseAlreadyCreated { database_name: String },
    #[error("The given database was not found: \"{database_name}\"")]
    DatabaseNotFound { database_name: String },
    #[error("Ran into a database error : \"{0}\"")]
    DatabaseError(#[from] sqlx::Error),
}

#[derive(Debug, Error)]
/// Represent databases errors
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
}

impl From<sqlx::Error> for GraphDbRuntimeError {
    fn from(val: sqlx::Error) -> Self {
        sqlx_error_to_db_error(val)
    }
}

pub fn sqlx_error_to_db_error(e: sqlx::Error) -> GraphDbRuntimeError {
    match &e {
        sqlx::Error::Database(database_error) => match database_error.kind() {
            sqlx::error::ErrorKind::UniqueViolation => todo!(),
            sqlx::error::ErrorKind::ForeignKeyViolation => todo!(),
            sqlx::error::ErrorKind::NotNullViolation => todo!(),
            sqlx::error::ErrorKind::CheckViolation => todo!(),
            sqlx::error::ErrorKind::Other => {
                let e_str = e.to_string();
                if e_str.contains("table") && e_str.contains("already exists") {
                    GraphDbRuntimeError::TableAlreadyCreatedError(e)
                } else if e_str.contains("syntax") {
                    GraphDbRuntimeError::QueryError(e)
                } else {
                    e.into()
                }
            }
            _ => todo!(),
        },
        _ => {
            error!("Sqlx error not handled : {}", e);
            e.into()
        }
    }
}
