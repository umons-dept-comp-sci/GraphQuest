use sqlx::{FromRow, MySql, Pool, mysql::MySqlDatabaseError};

use crate::database_handler::{ColumnType, DbQuerySystem, GraphDatabase, GraphDbRuntimeError};

/// An alias for [`GraphDatabase`] specialized for MySql
pub type MySqlGraphDB = GraphDatabase<MySql>;

impl DbQuerySystem<MySql> for MySql {
    async fn execute_query_no_return(
        pool: &Pool<MySql>,
        mut query_builder: sqlx::QueryBuilder<'_, MySql>,
    ) -> Result<(), GraphDbRuntimeError> {
        let query = query_builder.build();

        query.execute(pool).await?;
        Ok(())
    }

    async fn execute_query_fetch_all<V>(
        pool: &Pool<MySql>,
        mut query_builder: sqlx::QueryBuilder<'_, MySql>,
    ) -> Result<Vec<V>, GraphDbRuntimeError>
    where
        V: for<'r> FromRow<'r, <MySql as sqlx::Database>::Row>
            + std::marker::Send
            + std::marker::Unpin,
    {
        let query = query_builder.build_query_as();

        Ok(query.fetch_all(pool).await?)
    }

    async fn execute_query_fetch_one<V>(
        pool: &Pool<MySql>,
        mut query_builder: sqlx::QueryBuilder<'_, MySql>,
    ) -> Result<V, GraphDbRuntimeError>
    where
        V: for<'r> FromRow<'r, <MySql as sqlx::Database>::Row>
            + std::marker::Send
            + std::marker::Unpin,
    {
        let query = query_builder.build_query_as();

        Ok(query.fetch_one(pool).await?)
    }

    fn execute_query_fetch<'e, V>(
        pool: &'e Pool<MySql>,
        query_builder: &'e mut sqlx::QueryBuilder<'_, MySql>,
    ) -> std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<V, sqlx::Error>> + Send + 'e>>
    where
        V: for<'r> FromRow<'r, <MySql as sqlx::Database>::Row>
            + std::marker::Send
            + std::marker::Unpin
            + 'e,
    {
        let query = query_builder.build_query_as();

        query.fetch(pool)
    }

    fn execute_query_fetch_sql_rows<'e>(
        pool: &'e Pool<MySql>,
        query: sqlx::query::Query<'e, MySql, <MySql as sqlx::Database>::Arguments<'e>>,
    ) -> std::pin::Pin<
        Box<
            dyn tokio_stream::Stream<Item = Result<<MySql as sqlx::Database>::Row, sqlx::Error>>
                + Send
                + 'e,
        >,
    > {
        query.fetch(pool)
    }

    fn get_all_tables_query() -> String {
        todo!()
    }

    fn get_create_table_value_query(
        _table_name: impl ToString,
        _pk_column_name: impl ToString,
        _pk_column_type: ColumnType,
        _value_column_name: impl ToString,
        _value_column_type: ColumnType,
    ) -> String {
        todo!()
    }

    fn get_delete_table_query() -> String {
        todo!()
    }

    fn get_insert_into_query(
        _table_name: impl ToString,
        _nb_cols: usize,
        _nb_rows: usize,
    ) -> String {
        todo!()
    }

    fn get_is_table_present(_table_name: impl ToString) -> String {
        todo!()
    }

    fn get_create_table_query(
        _table_name: impl ToString,
        _pk_column_name: impl ToString,
        _pk_column_type: ColumnType,
    ) -> String {
        todo!()
    }

    fn to_sql(_query: &super::SqlSelectQuery) -> String {
        todo!()
    }
}

impl From<MySqlDatabaseError> for GraphDbRuntimeError {
    fn from(val: MySqlDatabaseError) -> Self {
        let error: sqlx::Error = val.into();
        error.into()
    }
}
