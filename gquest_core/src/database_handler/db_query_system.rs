use std::{
    collections::HashSet,
    fmt::{Debug, Display},
    pin::Pin,
};

use sqlx::{Database, FromRow, Pool, QueryBuilder, query::Query};
use tokio_stream::Stream;

use crate::database_handler::{GraphDbRuntimeError, GraphDbStartupError};

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

/// Represents a condition that could appear in a where clause.
#[derive(Debug, Clone, PartialEq)]
pub enum SqlCondition {
    /// A simple comparison
    Operation(SqlComparison),
    /// `x` or `y`
    Or(Box<SqlCondition>, Box<SqlCondition>),
    /// `x` and `y`
    And(Box<SqlCondition>, Box<SqlCondition>),
    /// `x_0` and `x_1` and ... and `x_{n-1}`.
    AndVec(Box<SqlCondition>, Vec<SqlCondition>),
    /// `x_0` or `x_1` or ... or `x_{n-1}`.
    OrVec(Box<SqlCondition>, Vec<SqlCondition>),
    /// not `x`
    Not(Box<SqlCondition>),
    /// exists `x`
    Exists(Box<SqlSelectQuery>),
}

impl SqlCondition {
    /// Simplifies the creation of the [`SqlCondition::And`] enum.
    pub fn and(cond_1: impl Into<SqlCondition>, cond_2: impl Into<SqlCondition>) -> Self {
        Self::And(Box::new(cond_1.into()), Box::new(cond_2.into()))
    }

    /// Simplifies the creation of the [`SqlCondition::Or`] enum.
    pub fn or(cond_1: impl Into<SqlCondition>, cond_2: impl Into<SqlCondition>) -> Self {
        Self::Or(Box::new(cond_1.into()), Box::new(cond_2.into()))
    }

    /// Simplifies the creation of the [`SqlCondition::Not`] enum.
    pub fn not(cond: impl Into<SqlCondition>) -> Self {
        Self::Not(Box::new(cond.into()))
    }

    /// Simplifies the creation of the [`SqlCondition::Exists`] enum.
    pub fn exists(query: impl Into<SqlSelectQuery>) -> Self {
        Self::Exists(Box::new(query.into()))
    }

    /// Simplifies the creation of the [`SqlCondition::AndVec`] enum.
    ///
    /// If `conds` is empty, then `cond_1` will simply be returned.
    pub fn and_vec(cond_1: impl Into<SqlCondition>, conds: Vec<impl Into<SqlCondition>>) -> Self {
        if conds.is_empty() {
            cond_1.into()
        } else {
            Self::AndVec(
                Box::new(cond_1.into()),
                conds.into_iter().map(|c| c.into()).collect(),
            )
        }
    }

    /// Simplifies the creation of the [`SqlCondition::OrVec`] enum.
    ///
    /// If `conds` is empty, then `cond_1` will simply be returned.
    pub fn or_vec(cond_1: impl Into<SqlCondition>, conds: Vec<impl Into<SqlCondition>>) -> Self {
        if conds.is_empty() {
            cond_1.into()
        } else {
            Self::OrVec(
                Box::new(cond_1.into()),
                conds.into_iter().map(|c| c.into()).collect(),
            )
        }
    }

    /// Adds the given prefix to all [`ArgType::IdentifierName`] present inside this comparison.
    pub fn add_prefix_identifier(&mut self, prefix: impl ToString) {
        let prefix = prefix.to_string();
        match self {
            SqlCondition::Operation(sql_comparison) => {
                sql_comparison.add_prefix_identifier(prefix);
            }
            SqlCondition::Or(sql_condition, sql_condition1)
            | SqlCondition::And(sql_condition, sql_condition1) => {
                sql_condition.add_prefix_identifier(&prefix);
                sql_condition1.add_prefix_identifier(prefix);
            }
            SqlCondition::AndVec(sql_condition, sql_conditions)
            | SqlCondition::OrVec(sql_condition, sql_conditions) => {
                sql_condition.add_prefix_identifier(&prefix);
                sql_conditions
                    .iter_mut()
                    .for_each(|cond| cond.add_prefix_identifier(&prefix));
            }
            SqlCondition::Not(sql_condition) => {
                sql_condition.add_prefix_identifier(prefix);
            }
            SqlCondition::Exists(_) => {
                todo!("Renaming is not yet implemented for select queries")
            }
        }
    }

    /// Searches recursively in the given condition for any column name.
    pub fn get_all_identifiers(&self) -> HashSet<String> {
        let mut res: HashSet<String> = HashSet::new();
        match &self {
            SqlCondition::Operation(sql_comparison) => match sql_comparison {
                SqlComparison::Greater(a, b)
                | SqlComparison::GreaterEqual(a, b)
                | SqlComparison::Less(a, b)
                | SqlComparison::LessEqual(a, b)
                | SqlComparison::Equal(a, b)
                | SqlComparison::NotEqual(a, b) => {
                    if let ArgType::Identifier(name) = a {
                        res.insert(name.to_string());
                    }
                    if let ArgType::Identifier(name) = b {
                        res.insert(name.to_string());
                    }
                }
            },
            SqlCondition::And(sql_condition, sql_condition1)
            | SqlCondition::Or(sql_condition, sql_condition1) => {
                res.extend(sql_condition.get_all_identifiers());
                res.extend(sql_condition1.get_all_identifiers());
            }
            SqlCondition::Not(sql_condition) => {
                res.extend(sql_condition.get_all_identifiers());
            }
            SqlCondition::AndVec(sql_condition, sql_conditions)
            | SqlCondition::OrVec(sql_condition, sql_conditions) => {
                res.extend(sql_condition.get_all_identifiers());
                for condition in sql_conditions {
                    res.extend(condition.get_all_identifiers());
                }
            }
            SqlCondition::Exists(_sql_select_query) => {
                todo!(
                    "Conditions using sql selections are not yet supported since finding identifiers names is harder here."
                )
            }
        };

        res
    }

    pub fn to_sql<DB>(&self) -> String
    where
        DB: Database + DbQuerySystem<DB>,
    {
        let concat =
            |cond1: &SqlCondition, operator: &str, sql_conditions: &Vec<SqlCondition>| -> String {
                let mut res = cond1.to_sql::<DB>().to_string();

                for condition in sql_conditions {
                    res.push_str(&format!(" {operator} {}", condition.to_sql::<DB>()));
                }

                res
            };

        match self {
            SqlCondition::Operation(sql_comparison) => sql_comparison.to_string(),
            SqlCondition::And(a, b) => format!("({}) AND ({})", a.to_sql::<DB>(), b.to_sql::<DB>()),
            SqlCondition::Or(a, b) => format!("({}) OR ({})", a.to_sql::<DB>(), b.to_sql::<DB>()),
            SqlCondition::Not(a) => format!("NOT ({})", a.to_sql::<DB>()),
            SqlCondition::AndVec(cond1, sql_conditions) => concat(cond1, "AND", sql_conditions),
            SqlCondition::OrVec(cond1, sql_conditions) => concat(cond1, "OR", sql_conditions),
            SqlCondition::Exists(sql_select_query) => {
                format!("EXISTS({})", DB::to_sql(sql_select_query))
            }
        }
    }
}

/// Represents a comparison that can be used in an Sql where clause.
/// Note that an [`SqlComparison`] is a [`SqlCondition`] and therefore can be turned into one.
#[derive(Debug, Clone, PartialEq)]
pub enum SqlComparison {
    /// `a > b`
    Greater(ArgType, ArgType),
    /// `a >= b`
    GreaterEqual(ArgType, ArgType),
    /// `a < b`
    Less(ArgType, ArgType),
    /// `a <= b`
    LessEqual(ArgType, ArgType),
    /// `a = b`
    Equal(ArgType, ArgType),
    /// `a != b`
    NotEqual(ArgType, ArgType),
}

/// Used to correctly identify arguments in a comparison,
/// otherwise it would be hard to guess if they refer to a value or to a column.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgType {
    Value(String),
    Identifier(String),
}

impl ArgType {
    pub fn value(value: impl ToString) -> Self {
        ArgType::Value(value.to_string())
    }
    pub fn identifier(name: impl ToString) -> Self {
        ArgType::Identifier(name.to_string())
    }
}

impl Display for ArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ArgType::Value(v) => format!("\'{v}\'"),
                ArgType::Identifier(v) => v.to_string(),
            }
        )
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
                SqlComparison::NotEqual(a, b) => format!("{a} != {b}"),
            }
        )
    }
}

impl From<SqlComparison> for SqlCondition {
    fn from(val: SqlComparison) -> Self {
        SqlCondition::Operation(val)
    }
}

impl SqlComparison {
    /// Adds the given prefix to the [`ArgType::IdentifierName`] present inside this comparison.
    pub fn add_prefix_identifier(&mut self, prefix: impl ToString) {
        let prefix = prefix.to_string();
        match self {
            SqlComparison::Greater(arg_type, arg_type1)
            | SqlComparison::GreaterEqual(arg_type, arg_type1)
            | SqlComparison::Less(arg_type, arg_type1)
            | SqlComparison::LessEqual(arg_type, arg_type1)
            | SqlComparison::Equal(arg_type, arg_type1)
            | SqlComparison::NotEqual(arg_type, arg_type1) => {
                if let ArgType::Identifier(name) = arg_type {
                    *name = format!("{prefix}{name}");
                }
                if let ArgType::Identifier(name) = arg_type1 {
                    *name = format!("{prefix}{name}");
                }
            }
        }
    }
}

/// Represents a table in the From section of an Sql Query.
#[derive(Debug, Clone, PartialEq)]
pub struct SqlTableSelection {
    /// The table to select
    pub selected_table: SqlTable,
    /// Contains all the table to join to this selected table  and the column name to use for each.
    pub join_clause: Option<(Vec<String>, String)>,
    /// What to rename the table in the From section of the query
    pub rename_as: Option<String>,
}

impl SqlTableSelection {
    /// Selects a simple table without joining anything to it and does not rename it.
    pub fn new(table: impl Into<SqlTable>) -> Self {
        Self {
            selected_table: table.into(),
            join_clause: None,
            rename_as: None,
        }
    }

    /// Selects a table and joins it with the given table name by using for each one the same common column name.
    pub fn new_join(
        table: impl Into<SqlTable>,
        with_tables: Vec<String>,
        using: impl ToString,
        rename_as: Option<String>,
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
impl From<String> for SqlTableSelection {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for SqlTableSelection {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Represents a table to select in Sql
#[derive(Debug, Clone, PartialEq)]
pub enum SqlTable {
    /// The query used to get this temporary table
    SqlQuery(SqlSelectQuery),
    /// The name of the table
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

/// Represents an SqlQuery that is general for any database system as it will be built for each one differently.
/// See [`DbQuerySystem::to_sql`] (or even [`SqlSelectQuery::to_sql`]) to understand how to translate it into a valid sql query.
#[derive(Debug, Clone, PartialEq)]
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
    /// Simply selects every row from the given table.
    /// In SQLite: `SELECT * FROM table`
    pub fn select_all_from_table(table: impl Into<SqlTableSelection>) -> Self {
        Self::select_column_from_table("*", table)
    }
    pub fn select_column_from_table(
        column_name: impl ToString,
        table: impl Into<SqlTableSelection>,
    ) -> Self {
        Self {
            select: vec![column_name.to_string()],
            from: vec![table.into()],
            where_clause: None,
            group_by: vec![],
            limit: None,
        }
    }

    /// Gets the number of rows stored inside the given table.
    /// In SQLite: `SELECT count(*) FROM table`
    pub fn select_count_all_from_table(table: impl Into<SqlTableSelection>) -> Self {
        Self::select_column_from_table("COUNT(*)", table)
    }

    /// Returns the *same* query but with the given **where** clause.
    pub fn set_where_clause(mut self, where_clause: impl Into<SqlCondition>) -> Self {
        self.where_clause = Some(where_clause.into());
        self
    }

    /// Encapsulates the previous conditions with an [`SqlCondition::And`] composed of the previous condition and the given one.
    ///
    /// And if no conditions were given before then it will be added directly.
    pub fn add_and(&mut self, additional_cond: impl Into<SqlCondition>) {
        if self.where_clause.is_some() {
            self.where_clause = Some(SqlCondition::and(
                self.where_clause.take().expect("is some"),
                additional_cond,
            ))
        } else {
            self.where_clause = Some(additional_cond.into());
        }
    }

    /// Adds a table to the **from** clause of the query.
    pub fn add_table(&mut self, table: impl Into<SqlTableSelection>) {
        self.from.push(table.into());
    }
    /// Adds all table to the **from** clause of the query.
    pub fn add_tables(&mut self, tables: Vec<impl Into<SqlTableSelection>>) {
        for table in tables {
            self.add_table(table);
        }
    }

    /// Returns the *same* query but with the given **limit/offset** clause.
    pub fn set_limit_clause(mut self, start: Option<usize>, limit: usize) -> Self {
        self.limit = Some((start, limit));
        self
    }

    /// Returns the *same* query but with the given **group by** clause.
    pub fn set_group_by(mut self, groups: Vec<impl ToString>) -> Self {
        self.group_by = groups.iter().map(|f| f.to_string()).collect();
        self
    }

    /// Using the given database system, converts this query to a valid and executable sql query for the given system.
    /// See [`DbQuerySystem::to_sql`] for more information.
    pub fn to_sql<DB>(&self) -> String
    where
        DB: Database + DbQuerySystem<DB>,
    {
        DB::to_sql(self)
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

    /// Returns the query that can be used to delete a table with the given name from the dataset
    fn get_delete_table_query(name: impl ToString) -> String;

    /// Turns the given query to a valid Sql query that could be executed using this database system.
    ///
    /// When implementing this function, **beware** that the `;` character should not be added at the end of the query since it could lead to issues when
    /// building queries recursively for example.
    fn to_sql(query: &SqlSelectQuery) -> String;

    /// Returns the query that can be used to insert all the given data into a table called `table_name`
    fn get_insert_into_query(table_name: impl ToString, nb_cols: usize, nb_rows: usize) -> String;

    /// Checks if the given table is present inside a database.
    /// Returns 1 if the table is present, 0 otherwise.
    fn get_is_table_present(table_name: impl ToString) -> String;

    fn translate_runtime_error(error: sqlx::Error) -> GraphDbRuntimeError;

    fn translate_startup_error(error: sqlx::Error) -> GraphDbStartupError;
}

pub trait ErrorTraduction<DB>
where
    DB: Database,
{
    fn to_graph_runtime_error(&self) -> GraphDbRuntimeError;
}