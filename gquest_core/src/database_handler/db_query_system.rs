use std::{fmt::Debug, pin::Pin};

use sqlx::{Database, FromRow, Pool, QueryBuilder, query::Query};
use tokio_stream::Stream;

use crate::database_handler::GraphDbRuntimeError;

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

/// A trait used to get all the necessary functions needed to execute queries on a specific database.
///
/// All values passed as arguments to the implemented functions will be safe to add to the query.
/// And all unsafe values will be taken care of by the function's callers, this is why some functions **will require** you to add some **binding symbols**,
/// which are symbols that will be replaced by user inputed values in a safe way (to prevent SQL injections for exemple).
///
/// Please refer to the following documentation for more informations : [`sqlx::query::Query::bind`].
pub trait DbQuerySystem<DB>: Debug
where
    DB: Database,
{
    /// Executes a query and discard the result (useful for inserting for example) but still checks for erros.
    fn execute_query_no_return(
        pool: &Pool<DB>,
        query_builder: QueryBuilder<'_, DB>,
    ) -> impl std::future::Future<Output = Result<(), GraphDbRuntimeError>> + Send;

    /// Executes a query and stores all the result in a vector
    fn execute_query_fetch_all<V>(
        pool: &Pool<DB>,
        query_builder: QueryBuilder<'_, DB>,
    ) -> impl std::future::Future<Output = Result<Vec<V>, GraphDbRuntimeError>> + Send
    where
        V: for<'r> FromRow<'r, DB::Row> + Send + Unpin;

    /// Executes a query and returns the first returned value (or an error if none are present).
    fn execute_query_fetch_one<V>(
        pool: &Pool<DB>,
        query_builder: QueryBuilder<'_, DB>,
    ) -> impl std::future::Future<Output = Result<V, GraphDbRuntimeError>> + Send
    where
        V: for<'r> FromRow<'r, DB::Row> + Send + Unpin;

    /// Executes a query and returns a stream of value.
    /// Useful to iterate over many values returned by a query without the risk of storing too many.
    fn execute_query_fetch<'e, V>(
        pool: &'e Pool<DB>,
        query_builder: &'e mut sqlx::QueryBuilder<'_, DB>,
    ) -> Pin<Box<dyn Stream<Item = Result<V, sqlx::Error>> + Send + 'e>>
    where
        V: for<'r> FromRow<'r, <DB as sqlx::Database>::Row> + Send + Unpin + 'e;

    /// Executes a query and returns a stream of sql Rows.
    /// Use this method when there is no information about the value returned by the query (i.e. number or content of columns not known at compile time)
    /// Useful to iterate over many values returned by a query without the risk of storing too many.
    fn execute_query_fetch_sql_rows<'e>(
        pool: &'e Pool<DB>,
        query: Query<'e, DB, DB::Arguments<'e>>,
    ) -> Pin<Box<dyn Stream<Item = Result<DB::Row, sqlx::Error>> + Send + 'e>>;

    /// Gets a query that returns all the table names from the database.
    fn get_all_tables_query() -> String;

    /// Returns the query that can be used to create a table with a name and the value column
    fn get_create_table_value_query(
        table_name: impl ToString,
        pk_column_name: impl ToString,
        pk_column_type: ColumnType,
        value_column_name: impl ToString,
        value_column_type: ColumnType,
    ) -> String;

    /// Returns the query that can be used to create a table with a name but no secondary column
    fn get_create_table_query(
        table_name: impl ToString,
        pk_column_name: impl ToString,
        pk_column_type: ColumnType
    ) -> String;

    /// Returns the query that can be used to get all value from a table with the given name from the dataset
    fn get_all_from_table(table_name: impl ToString) -> String;

    /// Returns the query that can be used to delete a table with the given name from the dataset
    fn get_delete_table_query() -> String;

    /// Returns the query that can be used to insert all the given data into a table called `table_name`
    fn get_insert_into_query(table_name: impl ToString, nb_cols: usize, nb_rows: usize) -> String;

    /// Get a query that can be used to join all the given tables using a common column.
    ///
    fn get_join_table_query(
        table_names: Vec<impl ToString>,
        common_column_name: impl ToString,
    ) -> String;

    /// Get a query that can be used to retrieve all rows from the given table and column
    fn get_all_rows_from_table_column(
        table_name: impl ToString,
        column_name: impl ToString,
    ) -> String;

    /// Get a query that can be used to retrieve the number of rows from the given table
    fn get_nb_rows_from_table(table_name: impl ToString) -> String;

    /// Get a query that can be used to select a batch from a given table.
    /// ## Args
    /// * `start_index` : The index of the table to start fetching the data at
    ///     * If the given value is `none`, the fetching will start a 0
    /// * `limit` : The limit on the number of value to fetch
    fn get_select_batch_from(
        from_table: String,
        start_index: Option<usize>,
        limit: usize,
    ) -> String;

    /// Checks if the given table is present inside a database.
    /// Returns 1 if the table is present, 0 otherwise.
    fn get_is_table_present(table_name: impl ToString) -> String;
}
