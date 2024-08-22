use core::fmt;

pub struct TableNotFoundError
{
    table_name: String
}

impl TableNotFoundError {
    pub fn new(table_name: String) -> Self
    {
        Self {
            table_name
        }
    }
}

impl fmt::Display for TableNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The given table name does not exist: {}", self.table_name)
    }
}


impl fmt::Debug for TableNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{ table: {} }}", self.table_name) // programmer-facing output
    }
}