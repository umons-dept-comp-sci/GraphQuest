use std::{
    fmt::{Debug, Display},
    pin::Pin,
};

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
    Float {
        default_value: Option<usize>,
    },
    Boolean {
        default_value: Option<bool>,
    },
}

pub enum SqlCondition {
    Operation(SqlComparison),
    And(Box<SqlCondition>, Box<SqlCondition>),
    Or(Box<SqlCondition>, Box<SqlCondition>),
    Not(Box<SqlCondition>),
}

impl SqlCondition {
    pub fn and(cond_1: impl Into<SqlCondition>, cond_2: impl Into<SqlCondition>) -> Self {
        Self::And(Box::new(cond_1.into()), Box::new(cond_2.into()))
    }
    pub fn or(cond_1: impl Into<SqlCondition>, cond_2: impl Into<SqlCondition>) -> Self {
        Self::Or(Box::new(cond_1.into()), Box::new(cond_2.into()))
    }
    pub fn not(cond: impl Into<SqlCondition>) -> Self {
        Self::Not(Box::new(cond.into()))
    }
}

impl Display for SqlCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SqlCondition::Operation(sql_comparison) => sql_comparison.to_string(),
                SqlCondition::And(a, b) => format!("({a}) AND ({b})"),
                SqlCondition::Or(a, b) => format!("({a}) OR ({b})"),
                SqlCondition::Not(a) => format!("NOT ({a})"),
            }
        )
    }
}

pub enum SqlComparison {
    /// `a > b`
    Greater(String, String),
    /// `a >= b`
    GreaterEqual(String, String),
    /// `a < b`
    Less(String, String),
    /// `a <= b`
    LessEqual(String, String),
    /// `a = b`
    Equal(String, String),
}

impl SqlComparison {
    pub fn greater(a: impl ToString, b: impl ToString) -> Self {
        Self::Greater(a.to_string(), b.to_string())
    }
    pub fn greater_equal(a: impl ToString, b: impl ToString) -> Self {
        Self::GreaterEqual(a.to_string(), b.to_string())
    }
    pub fn less(a: impl ToString, b: impl ToString) -> Self {
        Self::Less(a.to_string(), b.to_string())
    }
    pub fn less_equal(a: impl ToString, b: impl ToString) -> Self {
        Self::LessEqual(a.to_string(), b.to_string())
    }
    pub fn equal(a: impl ToString, b: impl ToString) -> Self {
        Self::Equal(a.to_string(), b.to_string())
    }
}

impl Display for SqlComparison {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SqlComparison::Greater(a, b) => format!("{a} > {b}"),
                SqlComparison::GreaterEqual(a, b) => format!("{a} >= {b}"),
                SqlComparison::Less(a, b) => format!("{a} < {b}"),
                SqlComparison::LessEqual(a, b) => format!("{a} <= {b}"),
                SqlComparison::Equal(a, b) => format!("{a} = {b}"),
            }
        )
    }
}

impl From<SqlComparison> for SqlCondition {
    fn from(val: SqlComparison) -> Self {
        SqlCondition::Operation(val)
    }
}

pub struct SqlTableSelection {
    pub selected_table: SqlTable,
    /// Contains all the table to join to the selected table
    pub join_clause: Option<(Vec<String>, String)>,
    pub rename_as: Option<String>,
}

impl SqlTableSelection {
    pub fn new(table: impl Into<SqlTable>) -> Self {
        Self {
            selected_table: table.into(),
            join_clause: None,
            rename_as: None,
        }
    }

    pub fn new_join(
        table: impl Into<SqlTable>,
        with_tables: Vec<String>,
        using: impl ToString,
        rename_as: Option<impl ToString>,
    ) -> Self {
        let mut rename = None;
        if let Some(new_name) = rename_as {
            rename = Some(new_name.to_string());
        }
        Self {
            selected_table: table.into(),
            join_clause: Some((with_tables, using.to_string())),
            rename_as: rename,
        }
    }
}

pub enum SqlTable {
    SqlQuery(SqlSelectQuery),
    TableName(String),
}
impl From<SqlSelectQuery> for SqlTable {
    fn from(val: SqlSelectQuery) -> Self {
        SqlTable::SqlQuery(val)
    }
}
impl From<String> for SqlTable {
    fn from(value: String) -> Self {
        SqlTable::TableName(value)
    }
}

impl From<&str> for SqlTable {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

pub struct SqlSelectQuery {
    /// Contains all the column to choose
    pub select: Vec<String>,
    /// Contains all the table to choose
    pub from: Vec<SqlTableSelection>,
    /// The condition to impose to the selection
    pub where_clause: Option<SqlCondition>,
    /// What to group the selection by
    pub group_by: Vec<String>,
    /// The limit of data to return.
    /// * The first value representing the optional starting point.
    /// * The second value representing the maximum number of rows to return.
    pub limit: Option<(Option<usize>, usize)>,
}
impl SqlSelectQuery {
    pub fn select_all_from_table(table: impl Into<SqlTableSelection>) -> Self {
        Self {
            select: vec!["*".to_string()],
            from: vec![table.into()],
            where_clause: None,
            group_by: vec![],
            limit: None,
        }
    }

    pub fn set_where_clause(mut self, where_clause: SqlCondition) -> Self {
        self.where_clause = Some(where_clause);
        self
    }

    pub fn set_limit_clause(mut self, start: Option<usize>, limit: usize) -> Self {
        self.limit = Some((start, limit));
        self
    }

    pub fn set_group_by(mut self, groups: Vec<impl ToString>) -> Self {
        self.group_by = groups.iter().map(|f| f.to_string()).collect();
        self
    }
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
        pk_column_type: ColumnType,
    ) -> String;

    fn build_select_query(query: &SqlSelectQuery) -> String;

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
