use crate::database_handler::{ColumnType, DbQuerySystem};

#[derive(Clone, Default, Debug)]
pub struct SqliteGraphDatabase {}

impl DbQuerySystem for SqliteGraphDatabase {
    fn get_all_tables_query() -> String {
        "SELECT name FROM sqlite_master WHERE type='table';".to_string()
    }

    fn get_create_table_query(
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

    fn get_delete_table_query() -> String {
        "DROP TABLE $".to_string()
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

    fn get_join_table_query(
        table_names: Vec<impl ToString>,
        common_column_name: impl ToString,
    ) -> String {
        let mut table_name_iterator = table_names.into_iter();
        let mut res = format!(
            "SELECT * FROM ({})",
            table_name_iterator
                .next()
                .expect("at least one val")
                .to_string()
        );
        for table_name in table_name_iterator {
            res.push_str(
                format!(
                    " INNER JOIN {} USING ({})",
                    table_name.to_string(),
                    common_column_name.to_string()
                )
                .as_str(),
            );
        }

        res
    }

    fn get_all_rows_from_table_column(
        table_name: impl ToString,
        column_name: impl ToString,
    ) -> String {
        format!(
            "SELECT {} FROM {}",
            table_name.to_string(),
            column_name.to_string()
        )
    }

    fn get_nb_rows_from_table(table_name: impl ToString) -> String {
        format!("SELECT count(*) FROM {};", table_name.to_string())
    }

    fn get_select_batch_from(
        from_table: String,
        start_index: Option<usize>,
        limit: Option<usize>,
    ) -> String {
        let start = start_index.unwrap_or(0);

        let mut res = format!("SELECT * FROM ({from_table})");

        if let Some(length) = limit {
            res.push_str(&format!(" LIMIT {length}"));
        }

        res.push_str(&format!(" OFFSET {start}"));

        res
    }

    fn get_all_from_table(table_name: impl ToString) -> String {
        format!("select * from {};", table_name.to_string())
    }

    fn get_is_table_present(table_name: impl ToString) -> String {
        format!(
            "SELECT count(name) FROM sqlite_master WHERE type='table' AND name='{}';",
            table_name.to_string()
        )
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
