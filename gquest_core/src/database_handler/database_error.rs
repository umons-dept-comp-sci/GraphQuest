use log::error;
use thiserror::Error;

#[derive(Debug, Error)]
/// Represent databases errors
pub enum GraphDatabaseError {
    #[error("The given database is already created \"{database_name}\"")]
    DatabaseAlreadyCreated { database_name: String },
    #[error("The given database was not found: \"{database_name}\"")]
    DatabaseNotFound { database_name: String },
    #[error(
        "An error happened related to the database : \"{database_name}\", reason : \"{reason}\""
    )]
    DatabaseError {
        database_name: String,
        reason: String,
    },
    #[error("The given table name does not exist \"{table_name}\"")]
    TableNotFoundError { table_name: String },
    #[error("The given table was already created: \"{table_name:?}\"")]
    TableAlreadyCreatedError { table_name: Option<String> },
    #[error("The following action is forbidden: \"{action}\"")]
    ForbiddenActionError { action: String },
    #[error("An unknown error happened with the following message: \"{error_message}\"")]
    UnknownError { error_message: String },
    #[error(
        "An error happened when trying to execute the query \"{query}\" : \"{error_message}\""
    )]
    QueryError {
        query: String,
        error_message: String,
    },
}

pub fn sqlx_error_to_db_error(e: sqlx::Error) -> GraphDatabaseError {
    match e {
        sqlx::Error::Database(database_error) => match database_error.kind() {
            sqlx::error::ErrorKind::UniqueViolation => todo!(),
            sqlx::error::ErrorKind::ForeignKeyViolation => todo!(),
            sqlx::error::ErrorKind::NotNullViolation => todo!(),
            sqlx::error::ErrorKind::CheckViolation => todo!(),
            sqlx::error::ErrorKind::Other => {
                GraphDatabaseError::TableAlreadyCreatedError { table_name: None }
            }
            _ => todo!(),
        },
        _ => {
            error!("Sqlx error not handled : {}", e);
            GraphDatabaseError::UnknownError {
                error_message: e.to_string(),
            }
        }
    }
}
