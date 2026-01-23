/// The maximum capacity of the vector before pushing and flushing its content
pub const BUFFER_VECTOR_MAX_SIZE: usize = 2000;
/// The name of the first created table of the dataset containing the initial dataset
pub const CANONICAL_TABLE_NAME: &str = "dataset";
/// The column name of the primary key of the dataset
pub const PK_NAME: &str = "canon";

/// The name of the vertices table of the dataset
pub const VERTICES_TABLE_NAME: &str = "n";

pub const INVARIANT_COLUMN_NAME: &str = "value";

/// The name of the table that has all the data
pub const FULL_TABLE_NAME: &str = "all_inv";

/// The name of a table used to rename temporarly a selection in a from clause
pub const TEMPORARY_TABLE_NAME: &str = "tempo_table";

/// The name of the table that has all the extremal graph
pub const EXTREMAL_TABLE_NAME: &str = "extremal";

/// The maximum size of a signature to store in the dataset
pub const SIGNATURE_MAX_SIZE: usize = 250;
/// The maximum size of a table name in the dataset
pub const TABLE_NAME_MAX_SIZE: usize = 250;
// /// The speed at which the observator will be notified (if any present on db)
// const ITERATION_BEFORE_NOTIFY: u8 = 10;

pub mod database_error;
use std::io::BufRead;

pub use database_error::*;

pub mod graph_database;
pub use graph_database::*;

pub mod sqlite_handler;
use log::info;
pub use sqlite_handler::*;

pub mod postgre_handler;
pub use postgre_handler::*;

pub mod db_query_system;
pub use db_query_system::*;

pub mod graph_queries;
pub use graph_queries::*;

use crate::utils::subject::Observer;

#[derive(Clone)]
/// Encapsulates all compatible [`GraphDb`].
pub enum AllowedGraphDb {
    Sqlite(SqliteGraphDB),
    Postgres(PgSqlGraphDB),
}

impl AllowedGraphDb {
    /// Attemps to connect to a database using the correct system based on the given URL content.
    /// If the given database does not exists, then it will be created.
    pub async fn connect_create_from_url(
        url: impl Into<String>,
        connection_options: Option<SqlxLogLevels>,
    ) -> Result<Self, GraphDbStartupError> {
        let url = url.into();
        if url.contains("sqlite") {
            Ok(Self::Sqlite(
                SqliteGraphDB::connect_create_graph_database(url, connection_options).await?,
            ))
        } else if url.contains("postgresql") {
            Ok(Self::Postgres(
                PgSqlGraphDB::connect_create_graph_database(url, connection_options).await?,
            ))
        } else {
            Err(GraphDbStartupError::UnknownDatabaseSystem { url })
        }
    }

    /// Attemps to connect to an existing database using the correct system based on the given URL content.
    pub async fn connect_from_url(
        url: impl Into<String>,
        connection_options: Option<SqlxLogLevels>,
    ) -> Result<Self, GraphDbStartupError> {
        let url = url.into();
        if url.contains("sqlite") {
            info!("Attempting to connect to an Sqlite database");
            Ok(Self::Sqlite(
                SqliteGraphDB::connect_graph_database(url, connection_options).await?,
            ))
        } else if url.contains("postgresql") {
            info!("Attempting to connect to a PostgreSQL database");
            Ok(Self::Postgres(
                PgSqlGraphDB::connect_graph_database(url, connection_options).await?,
            ))
        } else {
            Err(GraphDbStartupError::UnknownDatabaseSystem { url })
        }
    }

    /// Closes the connection to the given database.
    pub async fn close_connection(self) {
        match self {
            AllowedGraphDb::Sqlite(graph_database) => graph_database.close_connection().await,
            AllowedGraphDb::Postgres(graph_database) => graph_database.close_connection().await,
        }
    }

    /// Add all canonical signatures to the table [`CANONICAL_TABLE_NAME`] of the dabase.
    /// sSee [`GraphDatabase::add_to_dataset`] for more informations.
    pub async fn add_to_dataset(
        &mut self,
        reader: impl BufRead,
        batch_size: usize,
        optional_obs: Option<&mut dyn Observer>,
    ) -> Result<(), GraphDbRuntimeError> {
        match self {
            AllowedGraphDb::Sqlite(graph_database) => {
                graph_database
                    .add_to_dataset(reader, batch_size, optional_obs)
                    .await
            }
            AllowedGraphDb::Postgres(graph_database) => {
                graph_database
                    .add_to_dataset(reader, batch_size, optional_obs)
                    .await
            }
        }
    }
}

impl From<SqliteGraphDB> for AllowedGraphDb {
    fn from(value: SqliteGraphDB) -> Self {
        Self::Sqlite(value)
    }
}

impl From<PgSqlGraphDB> for AllowedGraphDb {
    fn from(value: PgSqlGraphDB) -> Self {
        Self::Postgres(value)
    }
}
