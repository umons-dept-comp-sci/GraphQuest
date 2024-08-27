use core::fmt;


pub enum GraphDatabaseError
{
    TableNotFoundError{
        table_name: String
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
            Self::TableNotFoundError { table_name } => f.debug_struct("TableNotFoundError").field("table_name", table_name).finish(),
        }
    }
}

impl GraphDatabaseError {
    /// Gets the error message of this error 
    fn get_error_message(&self) -> String
    {
        match self {
            GraphDatabaseError::TableNotFoundError { table_name } => format!("The given table name does not exist: {}", table_name),
        }
    }
}