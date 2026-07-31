use std::{fmt::Debug, pin::Pin};

use sqlx::{Database, FromRow, Pool, QueryBuilder, query::Query};
use tokio_stream::Stream;

use crate::{
    data_handler::{data_types::ConstantValue, module::TypedArg, rel_graph::FnArg},
    database_handler::{
        CANONICAL_TABLE_NAME, FUNCTION_OUTPUT_COL_NAME, GraphDbRuntimeError, GraphDbStartupError,
        PK_NAME,
    },
    parser::parsed_expression::{ArithmOp, Comparison, Condition, MathExpression},
};

// pub enum ColumnType {
//     String {
//         max_size: Option<usize>,
//         default_value: Option<String>,
//     },
//     Integer {
//         default_value: Option<usize>,
//     },
//     Float {
//         default_value: Option<usize>,
//     },
//     Boolean {
//         default_value: Option<bool>,
//     },
// }

#[derive(Debug, Clone, PartialEq)]
pub struct SqlWhereClause {
    not_exists: Option<Box<SqlSelectQuery>>,
    condition: Option<Condition<FnArg>>,
}

impl From<Condition<FnArg>> for SqlWhereClause {
    fn from(value: Condition<FnArg>) -> Self {
        SqlWhereClause {
            condition: Some(value),
            not_exists: None,
        }
    }
}

impl SqlWhereClause {
    pub fn from_condition(cond: Condition<FnArg>) -> Self {
        cond.into()
    }

    /// Creates a [`SqlWhereClause`] with a *Not Exists* clause.
    pub fn not_exists(query: impl Into<SqlSelectQuery>) -> Self {
        Self {
            not_exists: Some(Box::new(query.into())),
            condition: None,
        }
    }

    /// Sets the *Not Exists* clause of an [`SqlWhereClause`]. If one already existed, this will overwrite it with the new one.
    pub fn set_not_exists(&mut self, query: impl Into<SqlSelectQuery>) {
        self.not_exists = Some(Box::new(query.into()));
    }
}

impl Condition<FnArg> {
    fn to_sql<DB>(&self) -> String
    where
        DB: Database + DbQuerySystem<DB>,
    {
        // let concat = |cond1: &Condition<FnArg>,
        //               operator: &str,
        //               sql_conditions: &Vec<Condition<FnArg>>|
        //  -> String {
        //     let mut res = cond1.to_sql::<DB>().to_string();

        //     for condition in sql_conditions {
        //         res.push_str(&format!(" {operator} {}", condition.to_sql::<DB>()));
        //     }

        //     res
        // };

        match self {
            Condition::Operation(sql_comparison) => sql_comparison.to_sql::<DB>(),
            Condition::And(a, b) => {
                format!("({}) AND ({})", a.to_sql::<DB>(), b.to_sql::<DB>())
            }
            Condition::Or(a, b) => {
                format!("({}) OR ({})", a.to_sql::<DB>(), b.to_sql::<DB>())
            }
            Condition::Not(a) => format!("NOT ({})", a.to_sql::<DB>()),
        }
    }
}

impl SqlWhereClause {
    pub fn to_sql<DB>(&self) -> String
    where
        DB: Database + DbQuerySystem<DB>,
    {
        if let Some(sql_cond) = &self.condition {
            let sql_cond = sql_cond.to_sql::<DB>();
            match &self.not_exists {
                Some(not_exist_query) => {
                    format!(
                        "{sql_cond} AND NOT(EXISTS({}))",
                        DB::to_sql(not_exist_query)
                    )
                }
                None => sql_cond,
            }
        } else {
            match &self.not_exists {
                Some(not_exist_query) => {
                    format!("NOT(EXISTS({}))", DB::to_sql(not_exist_query))
                }
                None => "".to_string(),
            }
        }
    }
}

impl Comparison<FnArg> {
    fn to_sql<DB>(&self) -> String
    where
        DB: Database + DbQuerySystem<DB>,
    {
        match self {
            Comparison::Greater(a, b) => format!(
                "{} > {}",
                DB::translate_math_expr(a),
                DB::translate_math_expr(b)
            ),
            Comparison::GreaterEqual(a, b, epsilon) => match epsilon {
                Some(eps) => {
                    // a > b or |a - b| < epsilon
                    format!(
                        "({}) OR ({})",
                        Comparison::Greater(a.clone(), b.clone()).to_sql::<DB>(),
                        Comparison::Equal(a.clone(), b.clone(), Some(*eps)).to_sql::<DB>()
                    )
                }
                None => {
                    format!(
                        "{} >= {}",
                        DB::translate_math_expr(a),
                        DB::translate_math_expr(b)
                    )
                }
            },
            Comparison::Less(a, b) => format!(
                "{} < {}",
                DB::translate_math_expr(a),
                DB::translate_math_expr(b)
            ),
            Comparison::LessEqual(a, b, epsilon) => match epsilon {
                Some(eps) => {
                    // a < b or |a - b| < epsilon
                    format!(
                        "({}) OR ({})",
                        Comparison::Less(a.clone(), b.clone()).to_sql::<DB>(),
                        Comparison::Equal(a.clone(), b.clone(), Some(*eps)).to_sql::<DB>()
                    )
                }
                None => {
                    format!(
                        "{} <= {}",
                        DB::translate_math_expr(a),
                        DB::translate_math_expr(b)
                    )
                }
            },
            Comparison::Equal(a, b, epsilon) => match epsilon {
                Some(eps) => {
                    // |a - b| < eps
                    Comparison::Less(
                        MathExpression::abs(MathExpression::bin_operation(
                            a.clone(),
                            ArithmOp::Subtract,
                            b.clone(),
                        )),
                        FnArg::Constant(ConstantValue::Numeric(*eps)).into(),
                    )
                    .to_sql::<DB>()
                }
                None => {
                    format!(
                        "{} = {}",
                        DB::translate_math_expr(a),
                        DB::translate_math_expr(b)
                    )
                }
            },
            Comparison::NotEqual(a, b, epsilon) => match epsilon {
                Some(eps) => {
                    // |a - b| >= eps
                    Comparison::GreaterEqual(
                        MathExpression::abs(MathExpression::bin_operation(
                            a.clone(),
                            ArithmOp::Subtract,
                            b.clone(),
                        )),
                        FnArg::Constant(ConstantValue::Numeric(*eps)).into(),
                        None,
                    )
                    .to_sql::<DB>()
                }
                None => {
                    format!(
                        "{} != {}",
                        DB::translate_math_expr(a),
                        DB::translate_math_expr(b)
                    )
                }
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SqlJoin {
    table_name: String,
    alias_name: String,
    on_cond: Vec<Comparison<FnArg>>,
}

impl SqlJoin {
    pub fn new(
        table_name: impl Into<String>,
        alias_name: impl Into<String>,
        on_cond: Vec<Comparison<FnArg>>,
    ) -> Self {
        Self {
            table_name: table_name.into(),
            alias_name: alias_name.into(),
            on_cond,
        }
    }

    pub fn to_sql<DB>(&self) -> String
    where
        DB: Database + DbQuerySystem<DB>,
    {
        let mut on_clause = String::new();
        if !self.on_cond.is_empty() {
            on_clause.push_str(&self.on_cond.first().expect("present").to_sql::<DB>());
            for on in self.on_cond.iter().skip(1) {
                on_clause.push_str(&format!(" AND {}", on.to_sql::<DB>()));
            }
        }
        format!(
            "JOIN {} {} ON {}",
            self.table_name, self.alias_name, on_clause
        )
    }
}

/// Represents a table in the From section of an Sql Query.
#[derive(Debug, Clone, PartialEq)]
pub struct SqlTableSelection {
    /// The table to select
    pub selected_table: SqlTable,
    // /// Contains all the table to join to this selected table  and the column name to use for each.
    // pub join_clause: Option<(Vec<String>, String)>,
    /// What to rename the table in the From section of the query
    pub rename_as: Option<String>,
}

impl SqlTableSelection {
    /// Selects a simple table without joining anything to it and does not rename it.
    pub fn new(table: impl Into<SqlTable>) -> Self {
        Self {
            selected_table: table.into(),
            // join_clause: None,
            rename_as: None,
        }
    }

    /// Selects a simple table without joining anything to it and renames it.
    pub fn new_rename(table: impl Into<SqlTable>, new_name: impl ToString) -> Self {
        Self {
            selected_table: table.into(),
            // join_clause: None,
            rename_as: Some(new_name.to_string()),
        }
    }

    // /// Selects a table and joins it with the given table name by using for each one the same common column name.
    // pub fn new_join(
    //     table: impl Into<SqlTable>,
    //     with_tables: Vec<String>,
    //     using: impl ToString,
    //     rename_as: Option<String>,
    // ) -> Self {
    //     let mut rename = None;
    //     if let Some(new_name) = rename_as {
    //         rename = Some(new_name.to_string());
    //     }
    //     Self {
    //         selected_table: table.into(),
    //         join_clause: Some((with_tables, using.to_string())),
    //         rename_as: rename,
    //     }
    // }
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
    pub select: Vec<MathExpression<FnArg>>,
    pub distinct: bool,
    /// Contains all the table to choose
    pub from: Vec<SqlTableSelection>,
    /// The list of joins to add to the query.
    pub joins: Vec<SqlJoin>,
    /// The condition to impose to the selection
    pub where_clause: Option<SqlWhereClause>,
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
        Self::select_columns_from_table(
            vec![FnArg::Constant(ConstantValue::Identifier(column_name.to_string())).into()],
            table,
        )
    }

    pub fn select_columns_from_table(
        columns: Vec<MathExpression<FnArg>>,
        table: impl Into<SqlTableSelection>,
    ) -> Self {
        Self {
            select: columns,
            distinct: false,
            from: vec![table.into()],
            joins: Vec::new(),
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

    /// Returns the *same* query but with the given **where** clause, overwritting the previous one if it existed.
    pub fn set_where_clause(mut self, where_clause: impl Into<SqlWhereClause>) -> Self {
        self.where_clause = Some(where_clause.into());
        self
    }

    pub fn set_distinct_values(&mut self, value: bool) {
        self.distinct = value;
    }

    /// Encapsulates the previous conditions with an [`Condition::And`] composed of the previous condition and the given one.
    ///
    /// And if no conditions were given before then it will be added directly.
    pub fn add_and(&mut self, additional_cond: impl Into<Condition<FnArg>>) {
        if self.where_clause.is_some() {
            // We do this to keep the previous condition as well as the possible NotExists clause.
            let mut where_clause = self.where_clause.take().expect("is some");
            if where_clause.condition.is_some() {
                where_clause.condition = Some(Condition::and(
                    where_clause.condition.take().expect("is some"),
                    additional_cond,
                ));
            } else {
                where_clause.condition = Some(additional_cond.into())
            }
            self.where_clause = Some(where_clause);
        } else {
            self.where_clause = Some(additional_cond.into().into());
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

    pub fn add_join(&mut self, join: SqlJoin) {
        self.joins.push(join);
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

    /// Gets a query that can be used to create a table with the given name and columns.
    fn get_create_table_query(table_name: impl ToString, cols: &[TypedArg]) -> String;

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

    fn translate_constant(constant: &ConstantValue) -> String {
        match constant {
            ConstantValue::Numeric(num) => num.to_string(),
            ConstantValue::String(string) => format!("\'{string}\'"),
            ConstantValue::Bool(_) => todo!(
                "Bool can differ from function to function, this has to be revisited in the future !"
            ),
            ConstantValue::Identifier(id) => id.to_string(),
        }
    }

    fn translate_math_expr(expr: &MathExpression<FnArg>) -> String {
        match expr {
            MathExpression::Primitif(arg_type) => match arg_type {
                FnArg::FnCall(fn_ref) => {
                    // At this point, the function table must already be part of the join chain.
                    format!("{}{}.{FUNCTION_OUTPUT_COL_NAME}", fn_ref.0, fn_ref.1)
                }
                FnArg::Constant(constant_value) => Self::translate_constant(constant_value),
                // At this point, the dataset table must already be part of the join chain.
                FnArg::Dataset => format!("{CANONICAL_TABLE_NAME}.{PK_NAME}"),
            },
            MathExpression::Negation(math_expression) => {
                format!("-({})", Self::translate_math_expr(math_expression))
            }
            MathExpression::Floor(math_expression) => {
                format!("floor({})", Self::translate_math_expr(math_expression))
            }
            MathExpression::Ceil(math_expression) => {
                format!("ceil({})", Self::translate_math_expr(math_expression))
            }
            MathExpression::Abs(math_expression) => {
                format!("abs({})", Self::translate_math_expr(math_expression))
            }
            MathExpression::Sqrt(math_expression) => {
                format!("sqrt({})", Self::translate_math_expr(math_expression))
            }
            MathExpression::BinOperation { left, op, right } => {
                let left = Self::translate_math_expr(left);
                let right = Self::translate_math_expr(right);

                match op {
                    ArithmOp::Add => format!("({}) + ({})", left, right),
                    ArithmOp::Subtract => format!("({}) - ({})", left, right),
                    ArithmOp::Multiply => format!("({}) * ({})", left, right),
                    ArithmOp::Divide => format!("({}) / ({})", left, right),
                    ArithmOp::Power => format!("pow(({}), ({}))", left, right),
                    ArithmOp::Modulo => format!("mod(({}), ({}))", left, right),
                }
            }
        }
    }
}

pub trait ErrorTraduction<DB>
where
    DB: Database,
{
    fn to_graph_runtime_error(&self) -> GraphDbRuntimeError;
}
