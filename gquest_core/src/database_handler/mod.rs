/// The maximum capacity of the vector before pushing and flushing its content
pub const BUFFER_VECTOR_MAX_SIZE: usize = 2000;
/// The name of the first created table of the dataset containing the initial dataset
pub const CANONICAL_TABLE_NAME: &str = "Dataset";
/// The column name of the primary key of the dataset
pub const PK_NAME: &str = "canon";

/// The name of the vertices table of the dataset
pub const VERTICES_TABLE_NAME: &str = "vertices";

pub const INVARIANT_COLUMN_NAME: &str = "value";
/// The name of the table that has all the data
pub const FULL_TABLE_NAME: &str = "AllInv";

/// The maximum size of a signature to store in the dataset
pub const SIGNATURE_MAX_SIZE: usize = 250;
/// The maximum size of a table name in the dataset
pub const TABLE_NAME_MAX_SIZE: usize = 250;
// /// The speed at which the observator will be notified (if any present on db)
// const ITERATION_BEFORE_NOTIFY: u8 = 10;

pub mod database_error;
pub use database_error::*;

pub mod graph_database;
pub use graph_database::*;

pub mod sqlite_handler;
pub use sqlite_handler::*;

pub mod mysql_handler;
pub use mysql_handler::*;

pub mod db_query_system;
pub use db_query_system::*;
