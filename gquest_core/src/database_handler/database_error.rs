use core::fmt;

use log::error;

#[derive(Debug)]
/// Represent databases errors
pub enum GraphDatabaseError {
    /// Error to be raised when a database was already created
    DatabaseAlreadyCreated { database_name: String },
    /// Error to be raised when a database was not found
    DatabaseNotFound { database_name: String },
    /// Ran into an error when trying to connect to the database
    DatabaseError {
        database_name: String,
        reason: String,
    },
    /// Error to be raised when a table in a database was not found
    TableNotFoundError { table_name: String },
    /// Error to be raised when a table was already created in a database
    TableAlreadyCreatedError { table_name: Option<String> },
    /// Error to be raised when a forbidden action in the database was performed
    ForbiddenActionError { action: String },
    /// Error to be raised when an error not implemented was caught
    UnknownError { error_message: String },
    /// Error to be raised when the query paused a problem in the database
    QueryError {
        query: String,
        error_message: String,
    },
}

impl fmt::Display for GraphDatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_error_message())
    }
}

impl GraphDatabaseError {
    /// Gets the error message of this error
    fn get_error_message(&self) -> String {
        match self {
            GraphDatabaseError::TableNotFoundError { table_name } => {
                format!("The given table name does not exist: {}", table_name)
            }
            GraphDatabaseError::TableAlreadyCreatedError { table_name } => {
                format!("The given table was already created: {:?}", table_name)
            }
            GraphDatabaseError::ForbiddenActionError { action } => {
                format!("The following action is forbidden: {}", action)
            }
            GraphDatabaseError::DatabaseAlreadyCreated { database_name } => {
                format!("The given database is already created: {}", database_name)
            }
            GraphDatabaseError::DatabaseNotFound { database_name } => {
                format!("The given database was not found: {}", database_name)
            }
            GraphDatabaseError::UnknownError { error_message } => format!(
                "An unknown error happened with the following message: {}",
                error_message
            ),
            GraphDatabaseError::QueryError {
                query,
                error_message,
            } => format!(
                "An error happened when trying to execute the query {} : {}",
                query, error_message
            ),
            GraphDatabaseError::DatabaseError {
                database_name,
                reason,
            } => format!(
                "An error happened related to the database : \"{database_name}\", reason : \"{reason}\""
            ),
        }
    }
}

pub fn sqlx_error_to_db_error(e: sqlx::Error) -> GraphDatabaseError {
    match e {
        sqlx::Error::Database(database_error) => match database_error.kind() {
            sqlx::error::ErrorKind::UniqueViolation => todo!(),
            sqlx::error::ErrorKind::ForeignKeyViolation => todo!(),
            sqlx::error::ErrorKind::NotNullViolation => todo!(),
            sqlx::error::ErrorKind::CheckViolation => todo!(),
            sqlx::error::ErrorKind::Other => GraphDatabaseError::TableAlreadyCreatedError { table_name: None },
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
