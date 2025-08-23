use std::future::Future;

use sqlx::{
    query::{self, Query},
    sqlite::SqliteArguments,
    Pool, Sqlite,
};

use crate::{
    database_handler::{DatabaseType, GraphDatabase},
    utils::subject::{Observer, Subject},
};

#[derive(Clone)]
pub struct SqliteGraphDatabase {}


impl DatabaseType for SqliteGraphDatabase {
    fn get_all_tables_query(&self) -> String {
        todo!()
    }

    fn get_create_table_query(
        &self,
        table_name: &str,
        pk_name: &str,
        value_name: &str,
        value_type: super::ColumnType,
    ) -> String {
        todo!()
    }

    fn get_delete_table_query(&self, table_name: String) -> String {
        todo!()
    }

    fn get_insert_into_query(
        &self,
        table_name: String,
        signatures_values: &Vec<(String, String)>,
    ) -> String {
        todo!()
    }

    fn get_join_table_query(&self, table_names: Vec<String>, common_column_name: String) -> String {
        todo!()
    }

    fn get_all_rows_from_table_column(&self, table_name: &String, column_name: String) -> String {
        todo!()
    }

    fn get_select_batch_from(
        &self,
        from_table: String,
        start_index: Option<usize>,
        limit: Option<usize>,
    ) -> String {
        todo!()
    }
}