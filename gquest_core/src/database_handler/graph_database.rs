use log::debug;
use sqlx::{migrate::MigrateDatabase, Any, Pool, QueryBuilder, Sqlite};

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

pub struct GraphDatabase<'a> {
    pool: Pool<sqlx::Any>,
    obs: Option<&'a dyn Observer>,
    db_type: Box<dyn DatabaseType>,
}

/// The GraphDatabase trait is used to facilitate the communication with databases for the user.
impl<'a> GraphDatabase<'a> {
    async fn create_graph_database(db_url: &str) -> Result<Self, GraphDatabaseError> {
        // Creates the database if it didn't already exists
        if !sqlx::Any::database_exists(db_url).await.unwrap_or(false) {
            debug!("Creating database {}", db_url);
            match sqlx::Any::create_database(db_url).await {
                Ok(_) => {
                    debug!("Success creating the database {}", db_url);
                }
                Err(error) => {
                    return Err(GraphDatabaseError::UnknownError {
                        error_message: error.to_string(),
                    });
                }
            }
        } else {
            return Err(GraphDatabaseError::DatabaseAlreadyCreated {
                database_name: db_url.to_string(),
            });
        }

        let res = Self::connect_graph_database(db_url).await;
        if let Err(e) = res {
            return Err(e);
        }

        // Return the created struct
        Ok(res.unwrap())
    }

    async fn test(&mut self) {
        let mut query = QueryBuilder::<sqlx::Any>::new(self.db_type.get_all_tables_query()).sql();
        // .execute(&self.pool)
        // .await
        // .unwrap();

        // Ok(())
    }
}

fn test() {
    // SIGNATURE_MAX_SIZE
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
