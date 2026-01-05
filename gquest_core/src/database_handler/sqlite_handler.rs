use sqlx::{FromRow, Pool, query::Query, sqlite::Sqlite};

use tokio_stream::Stream;

use crate::database_handler::{
    ColumnType, DbQuerySystem, GraphDatabase, GraphDbRuntimeError, SqlSelectQuery, SqlTable,
};

/// An alias for [`GraphDatabase`] specialized for Sqlite
pub type SqliteGraphDB = GraphDatabase<Sqlite>;

impl DbQuerySystem<Sqlite> for Sqlite {
    async fn execute_query_no_return(
        pool: &Pool<Sqlite>,
        mut query_builder: sqlx::QueryBuilder<'_, Sqlite>,
    ) -> Result<(), GraphDbRuntimeError> {
        let query = query_builder.build();

        match query.execute(pool).await {
            Ok(_) => Ok(()),
            Err(e) => Err(Self::translate_runtime_error(e)),
        }
    }

    async fn execute_query_fetch_all<V>(
        pool: &Pool<Sqlite>,
        mut query_builder: sqlx::QueryBuilder<'_, Sqlite>,
    ) -> Result<Vec<V>, GraphDbRuntimeError>
    where
        V: for<'r> FromRow<'r, <Sqlite as sqlx::Database>::Row>
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
        pool: &Pool<Sqlite>,
        mut query_builder: sqlx::QueryBuilder<'_, Sqlite>,
    ) -> Result<V, GraphDbRuntimeError>
    where
        V: for<'r> FromRow<'r, <Sqlite as sqlx::Database>::Row>
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
        pool: &'e Pool<Sqlite>,
        query_builder: &'e mut sqlx::QueryBuilder<'_, Sqlite>,
    ) -> std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<V, sqlx::Error>> + Send + 'e>>
    where
        V: for<'r> FromRow<'r, <Sqlite as sqlx::Database>::Row>
            + std::marker::Send
            + std::marker::Unpin
            + 'e,
    {
        let query = query_builder.build_query_as();

        query.fetch(pool)
    }

    fn execute_query_fetch_sql_rows<'e>(
        pool: &'e Pool<Sqlite>,
        query: Query<'e, Sqlite, <sqlx::Sqlite as sqlx::Database>::Arguments<'e>>,
    ) -> std::pin::Pin<
        Box<dyn Stream<Item = Result<<Sqlite as sqlx::Database>::Row, sqlx::Error>> + Send + 'e>,
    > {
        query.fetch(pool)
    }

    fn get_all_tables_query() -> String {
        "SELECT name FROM sqlite_master WHERE type='table';".to_string()
    }

    fn get_create_table_value_query(
        table_name: impl ToString,
        pk_column_name: impl ToString,
        pk_column_type: ColumnType,
        value_column_name: impl ToString,
        value_column_type: ColumnType,
    ) -> String {
        format!(
            "CREATE TABLE {} ({} {} PRIMARY KEY NOT NULL, {} {})",
            table_name.to_string(),
            pk_column_name.to_string(),
            translate_column(pk_column_type),
            value_column_name.to_string(),
            translate_column(value_column_type)
        )
    }

    fn get_create_table_query(
        table_name: impl ToString,
        pk_column_name: impl ToString,
        pk_column_type: ColumnType,
    ) -> String {
        format!(
            "CREATE TABLE {} ({} {} PRIMARY KEY NOT NULL)",
            table_name.to_string(),
            pk_column_name.to_string(),
            translate_column(pk_column_type)
        )
    }

    fn to_sql(query: &SqlSelectQuery) -> String {
        let mut res = "SELECT ".to_string();

        // Add select clause
        for column_i in 0..query.select.len() - 1 {
            res.push_str(&format!("{}, ", query.select[column_i]));
        }
        res.push_str(&format!(
            "{} FROM ",
            query.select.last().expect("at least one val")
        ));

        // From clause
        for (i, table) in query.from.iter().enumerate() {
            res.push('(');
            match &table.selected_table {
                SqlTable::SqlQuery(sql_select_query) => {
                    res.push_str(&Self::to_sql(sql_select_query));
                }
                SqlTable::TableName(name) => res.push_str(name),
            }
            // Join query
            if let Some((tables_to_join, using)) = &table.join_clause {
                for table_name in tables_to_join {
                    res.push_str(&format!(" INNER JOIN {table_name} USING ({using})",));
                }
            }

            res.push(')');

            if let Some(alias) = &table.rename_as {
                res.push_str(&format!(" as {alias}"));
            }

            if i != query.from.len() - 1 {
                res.push_str(", ");
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

    fn get_insert_into_query(table_name: impl ToString, nb_rows: usize, nb_value: usize) -> String {
        let mut query = format!("INSERT OR REPLACE INTO {} VALUES ", table_name.to_string());

        for _ in 0..nb_value {
            let mut line = "(".to_string();
            for _ in 0..nb_rows {
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
        query.push(';');

        query
    }

    fn get_is_table_present(table_name: impl ToString) -> String {
        format!(
            "SELECT count(name) FROM sqlite_master WHERE type='table' AND name='{}';",
            table_name.to_string()
        )
    }

    fn get_delete_table_query(name: impl ToString) -> String {
        format!("DROP TABLE {}", name.to_string())
    }

    fn translate_runtime_error(error: sqlx::Error) -> GraphDbRuntimeError {
        match &error {
            sqlx::Error::Database(database_error) => match database_error.kind() {
                sqlx::error::ErrorKind::UniqueViolation
                | sqlx::error::ErrorKind::ForeignKeyViolation
                | sqlx::error::ErrorKind::NotNullViolation
                | sqlx::error::ErrorKind::CheckViolation => {
                    GraphDbRuntimeError::ViolationError(error)
                }
                sqlx::error::ErrorKind::Other => {
                    let e_str = error.to_string();
                    if e_str.contains("table") && e_str.contains("already exists") {
                        GraphDbRuntimeError::TableAlreadyCreatedError(error)
                    } else if e_str.contains("syntax") {
                        GraphDbRuntimeError::QueryError(error)
                    } else if e_str.contains("no such table:") {
                        // Error looks like this : `[error details here] no such table: [TABLE NAME]`
                        let mut e_str_iter = e_str.split("no such table:");
                        e_str_iter.next(); // Skip 

                        let table_name = e_str_iter
                            .next()
                            .expect("table name present")
                            .trim()
                            .to_string();
                        GraphDbRuntimeError::TableNotFoundError { table_name }
                    } else {
                        GraphDbRuntimeError::UnknownError(error)
                    }
                }
                _ => unreachable!("This is not suppose to be reached"),
            },
            _ => GraphDbRuntimeError::UnknownError(error),
        }
    }

    fn translate_startup_error(error: sqlx::Error) -> super::GraphDbStartupError {
        // TODO: Check correct error codes
        super::GraphDbStartupError::DatabaseError(error)
    }
}

fn translate_column(column_type: ColumnType) -> String {
    match column_type {
        ColumnType::String {
            max_size,
            default_value,
        } => {
            let mut tmp = String::from("VARCHAR");
            if let Some(m) = max_size {
                tmp.push_str(format!("({})", m).as_str());
            }
            if let Some(d) = default_value {
                tmp.push_str(format!(" DEFAULT {}", d).as_str());
            }
            tmp
        }
        ColumnType::Integer { default_value } => {
            let mut tmp = String::from("INTEGER");

            if let Some(d) = default_value {
                tmp.push_str(format!(" DEFAULT {}", d).as_str());
            }
            tmp
        }
        ColumnType::Float { default_value } => {
            let mut tmp = String::from("REAL");

            if let Some(d) = default_value {
                tmp.push_str(format!(" DEFAULT {}", d).as_str());
            }
            tmp
        }
        ColumnType::Boolean { default_value } => {
            let mut tmp = String::from("INTEGER");
            // Convert true to 1 and false to 0
            if let Some(d) = default_value {
                tmp.push_str(format!(" DEFAULT {}", { if d { 1 } else { 0 } }).as_str());
            }
            tmp
        }
    }
}
