use std::fmt::Debug;


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

/// A trait used to get all the necessary queries to execute.
/// This is the trait to implement in order to add another database system to the crate.
///
/// All values passed as arguments to the implemented functions will be safe to add to the query.
/// And all unsafe values will be taken care of by the function's callers, this is why some functions **will require** you to add some **binding symbols**,
/// which are symbols that will be replaced by user inputed values in a safe way (to prevent SQL injections for exemple).
///
/// It is possible that the symbols depend on what database system you are currently implementing,
/// please refer to this documentation for more informations [`sqlx::query::Query::bind`].
pub trait DbQuerySystem: Debug {
    /// Gets a query that returns all the table names from the database.
    fn get_all_tables_query(&self) -> String;

    /// Returns the query that can be used to create a table with a name and the column
    /// # Bind Symbols
    /// The returned query needs to contain **4 bind symbols** for the following elements :
    /// * The name of the table
    /// * The name for the primary key column
    /// * The maximum size of the primary key 
    /// * The name for the value column
    fn get_create_table_query(&self, value_type: ColumnType) -> String;

    /// Returns the query that can be used to delete a table with the given name from the dataset
    fn get_delete_table_query(&self, table_name: String) -> String;

    /// Returns the query that can be used to insert all the given data into a table called `table_name`
    fn get_insert_into_query(
        &self,
        table_name: String,
        signatures_values: &[(String, String)],
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
