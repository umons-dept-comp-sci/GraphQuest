use crate::database_handler::{ColumnType, DbQuerySystem};

#[derive(Clone, Default, Debug)]
pub struct SqliteGraphDatabase {}

impl DbQuerySystem for SqliteGraphDatabase {
    fn get_all_tables_query() -> String {
        "SELECT name FROM sqlite_master WHERE type='table';".to_string()
    }

    fn get_create_table_query(pk_column_type: ColumnType, value_column_type: ColumnType) -> String {
        format!(
            "CREATE TABLE $ ($ {} PRIMARY KEY NOT NULL, $ {})",
            translate_column(pk_column_type),
            translate_column(value_column_type)
        )
    }

    fn get_delete_table_query(table_name: String) -> String {
        todo!()
    }

    fn get_insert_into_query(table_name: String, signatures_values: &[(String, String)]) -> String {
        todo!()
    }

    fn get_join_table_query(table_names: Vec<String>, common_column_name: String) -> String {
        todo!()
    }

    fn get_all_rows_from_table_column(table_name: &String, column_name: String) -> String {
        todo!()
    }

    fn get_select_batch_from(
        from_table: String,
        start_index: Option<usize>,
        limit: Option<usize>,
    ) -> String {
        todo!()
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
        ColumnType::Boolean { default_value } => {
            let mut tmp = String::from("INTEGER");
            // Convert true to 1 and false to 0
            if let Some(d) = default_value {
                tmp.push_str(
                    format!(" DEFAULT {}", {
                        if d {
                            1
                        } else {
                            0
                        }
                    })
                    .as_str(),
                );
            }
            tmp
        }
    }
}
