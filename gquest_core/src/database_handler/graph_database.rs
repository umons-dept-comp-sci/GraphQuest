use std::{str::FromStr, time::Duration};

use log::debug;
use sqlx::{
    any::AnyConnectOptions, migrate::MigrateDatabase, pool, Any, AnyPool, ConnectOptions, Pool,
    QueryBuilder, Sqlite,
};

use crate::{database_handler::GraphDatabaseError, utils::subject::Observer};

pub enum ColumnType {
    String {
        max_size: Option<usize>,
        default_value: Option<String>,
    },
    Integer {
        default_value: Option<usize>,
    },
    Boolean {
        default_value: Option<bool>,
    },
}

/// Used to change the logging settings of a sqlx connection.
/// This is because by default, all debug options will be shown and will fill the logger really quickly. It is really recommended to set them to at least [`log::LevelFilter::Info`].
pub struct SqlxLogLevels {
    pub log_statement_level: Option<log::LevelFilter>,
    pub log_slow_statement_level: Option<(log::LevelFilter, Duration)>,
}

/// Represents a graph database.
/// As long as it is not dropped, the connection to the related database will be stay up.
pub struct GraphDatabase<'a, T>
where
    T: DatabaseType,
{
    pool: Pool<sqlx::Any>,
    obs: Option<&'a dyn Observer>,
    db_type: T,
}

/// The GraphDatabase trait is used to facilitate the communication with databases for the user.
impl<'a, T: DatabaseType> GraphDatabase<'a, T> {
    /// Creates the database that will be storing the project.
    /// Returns an in instance of a [GraphDatabase].
    ///
    /// ## Errors
    ///
    /// * [GraphDatabaseError::DatabaseAlreadyCreated] if the database was already created
    /// * [GraphDatabaseError::DatabaseError] if an unknown error was uncountered when trying to create it
    pub async fn create_graph_database(
        db_url: &str,
        db_type: T,
        connection_options: Option<SqlxLogLevels>,
    ) -> Result<Self, GraphDatabaseError> {
        // Install sqlite, postgre and mysql drivers
        sqlx::any::install_default_drivers();

        // Creates the database if it didn't already exists
        if !sqlx::Any::database_exists(db_url).await.unwrap_or(false) {
            debug!("Creating database {}", db_url);
            match sqlx::Any::create_database(db_url).await {
                Ok(_) => {
                    debug!("Success creating the database {}", db_url);
                }
                Err(error) => {
                    return Err(GraphDatabaseError::DatabaseError {
                        database_name: db_url.to_string(),
                        reason: error.to_string(),
                    });
                }
            }
        } else {
            return Err(GraphDatabaseError::DatabaseAlreadyCreated {
                database_name: db_url.to_string(),
            });
        }

        Self::connect_graph_database(db_url, db_type, connection_options).await
    }

    /// Connects to the given database.
    /// Returns an in instance of a [GraphDatabase].
    ///
    /// ## Errors
    ///
    /// Returns a:
    /// *   [GraphDatabaseError::DatabaseError] if something went wrong during the connection.
    pub async fn connect_graph_database(
        db_url: &str,
        db_type: T,
        connection_options: Option<SqlxLogLevels>,
    ) -> Result<Self, GraphDatabaseError> {
        // Install sqlite, postgre and mysql drivers
        sqlx::any::install_default_drivers();
        Ok(GraphDatabase {
            db_type,
            obs: None,
            pool: {
                let res = match connection_options {
                    Some(opt) => connect_with_options(db_url, opt).await,
                    None => sqlx::AnyPool::connect(db_url).await,
                };
                match res {
                    Ok(p) => p,
                    Err(e) => {
                        return Err(GraphDatabaseError::DatabaseError {
                            database_name: db_url.to_string(),
                            reason: e.to_string(),
                        })
                    }
                }
            },
        })
    }

    /// Closes the connection with the database (and drops this [`GraphDatabase`] instance)
    pub async fn close_connection(self) {
        self.pool.close().await
    }

    // async fn test(&mut self) {
    //     let mut query = QueryBuilder::<sqlx::Any>::new(self.db_type.get_all_tables_query()).sql();
    //     // .execute(&self.pool)
    //     // .await
    //     // .unwrap();

    //     // Ok(())
    // }
}

/// Applies the given options to the pool *before* opening it.
async fn connect_with_options(
    url: &str,
    log_levels: SqlxLogLevels,
) -> Result<AnyPool, sqlx::Error> {
    let mut connect_opt = AnyConnectOptions::from_str(url)?;

    if let Some(level) = log_levels.log_statement_level {
        connect_opt = connect_opt.log_statements(level);
    }
    if let Some(level) = log_levels.log_slow_statement_level {
        connect_opt = connect_opt.log_slow_statements(level.0, level.1);
    }
    AnyPool::connect_with(connect_opt).await
}

pub trait DatabaseType {
    /// Gets a query that returns all the table names from the database
    fn get_all_tables_query(&self) -> String;

    /// Returns the query that can be used to create a table with the given table name,
    /// that has two columns also with the given names :
    /// * `pk_name`: The primary key column (must be able to contain strings with a maximum size of [`crate::database_handler::SIGNATURE_MAX_SIZE`]).
    /// * `value_name`: The column that will store values.
    fn get_create_table_query(
        &self,
        table_name: &str,
        pk_name: &str,
        value_name: &str,
        value_type: ColumnType,
    ) -> String;

    /// Returns the query that can be used to delete a table with the given name from the dataset
    fn get_delete_table_query(&self, table_name: String) -> String;

    /// Returns the query that can be used to insert all the given data into a table called `table_name`
    fn get_insert_into_query(
        &self,
        table_name: String,
        signatures_values: &Vec<(String, String)>,
    ) -> String;

    /// Get a query that can be used to join all the given tables using a common column
    fn get_join_table_query(&self, table_names: Vec<String>, common_column_name: String) -> String;

    /// Get a query that can be used to retrieve all rows from the given table and column
    fn get_all_rows_from_table_column(&self, table_name: &String, column_name: String) -> String;

    /// Get a query that can be used to select a batch from a given table
    /// ## Args
    /// * `start_index` : The index of the table to start fetching the data at
    ///     * If the given value is `none`, the fetching will start a 0
    /// * `limit` : The limit on the number of value to fetch
    ///     * If the given value is `none`, the fetching will be stop at the end of the table
    fn get_select_batch_from(
        &self,
        from_table: String,
        start_index: Option<usize>,
        limit: Option<usize>,
    ) -> String;
}
