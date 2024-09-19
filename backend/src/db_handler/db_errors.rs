use core::fmt;


pub enum GraphDatabaseError
{
    DatabaseAlreadyCreated{
        database_name: String
    },
    DatabaseNotFound{
        database_name: String,
    },
    TableNotFoundError{
        table_name: String
    },
    TableAlreadyCreatedError{
        table_name: String
    },
    ForbiddenActionError{
        action: String
    },
    UnknownError{
        error_message: String
    },
    QueryError{
        query: String,
        error_message: String
    }
}


impl fmt::Display for GraphDatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_error_message())
    }
}

impl fmt::Debug for GraphDatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphDatabaseError::TableNotFoundError { table_name } => f.debug_struct("TableNotFoundError").field("table_name", table_name).finish(),
            GraphDatabaseError::TableAlreadyCreatedError { table_name } => f.debug_struct("TableAlreadyCreatedError").field("table_name", table_name).finish(),
            GraphDatabaseError::ForbiddenActionError { action } => f.debug_struct("ForbiddenActionError").field("action", action).finish(),
            GraphDatabaseError::DatabaseAlreadyCreated { database_name } => f.debug_struct("DatabaseAlreadyCreated").field("database_name", database_name).finish(),
            GraphDatabaseError::DatabaseNotFound { database_name } => f.debug_struct("DatabaseNotFound").field("database_name", database_name).finish(),
            GraphDatabaseError::UnknownError { error_message } => f.debug_struct("UnknownError").field("error_message", error_message).finish(),
            GraphDatabaseError::QueryError { query, error_message } => f.debug_struct("QueryError").field("error_message", error_message).field("query", query).finish(),
        }
    }
}

impl GraphDatabaseError {
    /// Gets the error message of this error 
    fn get_error_message(&self) -> String
    {
        match self {
            GraphDatabaseError::TableNotFoundError { table_name } => format!("The given table name does not exist: {}", table_name),
            GraphDatabaseError::TableAlreadyCreatedError { table_name } => format!("The given table was already created: {}", table_name),
            GraphDatabaseError::ForbiddenActionError { action } => format!("The following action is forbidden: {}", action),
            GraphDatabaseError::DatabaseAlreadyCreated { database_name } => format!("The given database is already created: {}", database_name),
            GraphDatabaseError::DatabaseNotFound { database_name } => format!("The given database was not found: {}", database_name),
            GraphDatabaseError::UnknownError { error_message } => format!("An unknown error happened with the following message: {}", error_message),
            GraphDatabaseError::QueryError { query, error_message } => format!("An error happened when trying to execute the query {} : {}", query, error_message),
        }
    }
}