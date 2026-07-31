use sqlx::{FromRow, Pool, Postgres, postgres::PgDatabaseError};

use crate::{
    data_handler::{module::TypedArg, rel_graph::FnArg},
    database_handler::{
        DbQuerySystem, GraphDatabase, GraphDb, GraphDbRuntimeError, GraphDbStartupError,
        SqlSelectQuery, SqlTable,
    },
    parser::parsed_expression::{ArithmOp, MathExpression},
};

/// An alias for a [`GraphDatabase`] specialized for Postgres
pub type PgSqlGraphDB = GraphDatabase<Postgres>;

impl GraphDb for Postgres {}

impl DbQuerySystem<Postgres> for Postgres {
    async fn execute_query_no_return(
        pool: &Pool<Postgres>,
        mut query_builder: sqlx::QueryBuilder<'_, Postgres>,
    ) -> Result<(), GraphDbRuntimeError> {
        let query = query_builder.build();

        match query.execute(pool).await {
            Ok(_) => Ok(()),
            Err(e) => Err(Self::translate_runtime_error(e)),
        }
    }

    async fn execute_query_fetch_all<V>(
        pool: &Pool<Postgres>,
        mut query_builder: sqlx::QueryBuilder<'_, Postgres>,
    ) -> Result<Vec<V>, GraphDbRuntimeError>
    where
        V: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row>
            + std::marker::Send
            + std::marker::Unpin,
    {
        let query = query_builder.build_query_as();

        match query.fetch_all(pool).await {
            Ok(v) => Ok(v),
            Err(e) => Err(Self::translate_runtime_error(e)),
        }
    }

    async fn execute_query_fetch_one<V>(
        pool: &Pool<Postgres>,
        mut query_builder: sqlx::QueryBuilder<'_, Postgres>,
    ) -> Result<V, GraphDbRuntimeError>
    where
        V: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row>
            + std::marker::Send
            + std::marker::Unpin,
    {
        let query = query_builder.build_query_as();

        match query.fetch_one(pool).await {
            Ok(v) => Ok(v),
            Err(e) => Err(Self::translate_runtime_error(e)),
        }
    }

    fn execute_query_fetch<'e, V>(
        pool: &'e Pool<Postgres>,
        query_builder: &'e mut sqlx::QueryBuilder<'_, Postgres>,
    ) -> std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<V, sqlx::Error>> + Send + 'e>>
    where
        V: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row>
            + std::marker::Send
            + std::marker::Unpin
            + 'e,
    {
        let query = query_builder.build_query_as();

        query.fetch(pool)
    }

    fn execute_query_fetch_sql_rows<'e>(
        pool: &'e Pool<Postgres>,
        query: sqlx::query::Query<'e, Postgres, <Postgres as sqlx::Database>::Arguments<'e>>,
    ) -> std::pin::Pin<
        Box<
            dyn tokio_stream::Stream<Item = Result<<Postgres as sqlx::Database>::Row, sqlx::Error>>
                + Send
                + 'e,
        >,
    > {
        query.fetch(pool)
    }

    fn get_all_tables_query() -> String {
        "SELECT tablename FROM pg_catalog.pg_tables
WHERE schemaname != 'pg_catalog' AND
    schemaname != 'information_schema';"
            .to_string()
    }

    fn get_delete_table_query(name: impl ToString) -> String {
        format!("DROP TABLE {}", name.to_string().to_lowercase())
    }

    fn to_sql(query: &SqlSelectQuery) -> String {
        let mut res = "SELECT ".to_string();
        if query.distinct {
            res.push_str("DISTINCT ");
        }

        // Add select clause
        for column_i in 0..query.select.len() - 1 {
            res.push_str(&format!(
                "{}, ",
                Self::translate_math_expr(&query.select[column_i])
            ));
        }
        res.push_str(&format!(
            "{} FROM ",
            Self::translate_math_expr(query.select.last().expect("at least one val"))
        ));
        // From clause
        for (i, table) in query.from.iter().enumerate() {
            match &table.selected_table {
                SqlTable::SqlQuery(sql_select_query) => {
                    res.push('(');
                    res.push_str(&Self::to_sql(sql_select_query));
                    res.push(')');
                }
                SqlTable::TableName(name) => res.push_str(name),
            }
            // // Inner Join query
            // if let Some((tables_to_join, using)) = &table.join_clause {
            //     for table_name in tables_to_join {
            //         res.push_str(&format!(" INNER JOIN {table_name} USING ({using})",));
            //     }
            // }

            if let Some(alias) = &table.rename_as {
                res.push_str(&format!(" as {alias}"));
            }

            if i != query.from.len() - 1 {
                res.push_str(", ");
            }
        }

        // Join clauses
        if !query.joins.is_empty() {
            res.push(' ');
            for sql_join in &query.joins {
                res.push_str(&format!("{} ", sql_join.to_sql::<Self>()));
            }
        }

        // Where clause
        if let Some(where_clause) = &query.where_clause {
            res.push_str(&format!(" WHERE {}", where_clause.to_sql::<Self>()));
        }

        // Group by
        if !query.group_by.is_empty() {
            res.push_str(&format!(
                " GROUP BY {}",
                query.group_by.first().expect("not empty")
            ));
            for i in 1..query.group_by.len() {
                res.push_str(&format!(", {}", query.group_by[i]));
            }
        }

        // Limit clause
        if let Some((start, limit)) = &query.limit {
            res.push_str(&format!(" LIMIT {limit}"));

            if let Some(start) = start {
                res.push_str(&format!(" OFFSET {start}"));
            }
        }

        res
    }

    fn get_insert_into_query(table_name: impl ToString, nb_cols: usize, nb_rows: usize) -> String {
        let mut query = format!("INSERT INTO {} VALUES ", table_name.to_string());

        for _ in 0..nb_rows {
            let mut line = "(".to_string();
            for _ in 0..nb_cols {
                line.push_str("?, ");
            }
            // Remove extra `, `
            line.pop();
            line.pop();
            line.push_str("), ");
            query.push_str(&line);
        }
        // Remove extra `, `
        query.pop();
        query.pop();
        // UPSERT Query, update if any conflict is detected
        query.push_str(" ON CONFLICT DO NOTHING;");

        query
    }

    fn get_is_table_present(table_name: impl ToString) -> String {
        let table_name = table_name.to_string().to_lowercase();
        format!(
            "SELECT EXISTS (
   SELECT FROM pg_catalog.pg_class c
   JOIN   pg_catalog.pg_namespace n ON n.oid = c.relnamespace
   WHERE      c.relname = '{table_name}'
   AND    c.relkind = 'r'    -- only tables
   )::int;" // int is used to cast boolean response to 0 or 1
        )
    }

    fn translate_runtime_error(error: sqlx::Error) -> GraphDbRuntimeError {
        let pg_error: &PgDatabaseError = error
            .as_database_error()
            .expect("correct error")
            .downcast_ref();

        // println!("{error:?}");

        match pg_error.code() {
            "42P07" => GraphDbRuntimeError::TableAlreadyCreatedError(error),
            // 42703 => Missing column
            _ => GraphDbRuntimeError::UnknownError(error),
        }
    }

    fn translate_startup_error(error: sqlx::Error) -> GraphDbStartupError {
        println!("error : {error}");
        match error.as_database_error() {
            Some(e) => {
                let pg_error: &PgDatabaseError = e.downcast_ref();
                match pg_error.code() {
                    "42501" => GraphDbStartupError::MissingPrivilege(error),
                    _ => GraphDbStartupError::DatabaseError(error),
                }
            }
            None => GraphDbStartupError::DatabaseError(error),
        }
    }

    fn translate_math_expr(expr: &MathExpression<FnArg>) -> String {
        match expr {
            MathExpression::Primitif(arg_type) => todo!("Translate into postgre primitives"),
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
                    ArithmOp::Modulo => format!(
                        "mod(CAST(({}) AS numeric), CAST(({}) AS numeric))", // The mod function is the one forcing us to re-implement this function
                        left, right
                    ),
                }
            }
        }
    }

    fn get_create_table_query(table_name: impl ToString, cols: &[TypedArg]) -> String {
        todo!()
    }
}

// fn translate_column(column_type: ColumnType) -> String {
//     match column_type {
//         ColumnType::String {
//             max_size,
//             default_value,
//         } => {
//             let mut tmp = String::from("VARCHAR");
//             if let Some(m) = max_size {
//                 tmp.push_str(format!("({})", m).as_str());
//             }
//             if let Some(d) = default_value {
//                 tmp.push_str(format!(" DEFAULT {}", d).as_str());
//             }
//             tmp
//         }
//         ColumnType::Integer { default_value } => {
//             let mut tmp = String::from("INTEGER");

//             if let Some(d) = default_value {
//                 tmp.push_str(format!(" DEFAULT {}", d).as_str());
//             }
//             tmp
//         }
//         ColumnType::Float { default_value } => {
//             let mut tmp = String::from("float8");

//             if let Some(d) = default_value {
//                 tmp.push_str(format!(" DEFAULT {}", d).as_str());
//             }
//             tmp
//         }
//         ColumnType::Boolean { default_value } => {
//             let mut tmp = String::from("INTEGER");
//             // Convert true to 1 and false to 0
//             if let Some(d) = default_value {
//                 tmp.push_str(format!(" DEFAULT {}", { if d { 1 } else { 0 } }).as_str());
//             }
//             tmp
//         }
//     }
// }
