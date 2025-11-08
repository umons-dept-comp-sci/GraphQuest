use std::io::BufRead;
use std::{fmt::Debug, time::Duration};

use log::debug;
use sqlx::any::AnyRow;
use sqlx::{migrate::MigrateDatabase, pool::PoolOptions, Database, FromRow, Pool, QueryBuilder};
use sqlx::{Column, Row};
use tokio_stream::StreamExt;

use crate::database_handler::{DbQuerySystem, GraphDbRuntimeError, *};
use crate::utils::table_handler::QueryTable;

/// Used to change the logging settings of a sqlx connection.
/// This is because by default, all debug options will be shown and will fill the logger really quickly.
///  t is really recommended to set them to at least [`log::LevelFilter::Info`].
pub struct SqlxLogLevels {
    pub log_statement_level: Option<log::LevelFilter>,
    pub log_slow_statement_level: Option<(log::LevelFilter, Duration)>,
}

#[derive(Debug, Clone)]
/// Represents a graph database.
/// As long as it is not dropped, the connection to the related database will be stay up.
pub struct GraphDatabase<DB>
where
    DB: Database + DbQuerySystem<DB>,
{
    // _url: String,
    pool: Pool<DB>,
}

/// The GraphDatabase trait is used to facilitate the communication with databases for the user.
impl<DB: Database + DbQuerySystem<DB>> GraphDatabase<DB> {
    /// Creates the database that will be storing the project.
    /// Returns an in instance of a [GraphDatabase].
    ///
    /// ## Errors
    ///
    /// * [`GraphDbStartupError::DatabaseAlreadyCreated`] if the database was already created
    /// * [`GraphDbStartupError::DatabaseError`] if an unknown error was uncountered when trying to create it
    pub async fn create_graph_database(
        db_url: &str,
        connection_options: Option<SqlxLogLevels>,
    ) -> Result<Self, GraphDbStartupError> {
        // Install sqlite, postgre and mysql drivers
        sqlx::any::install_default_drivers();

        // Creates the database if it didn't already exists
        if !sqlx::Any::database_exists(db_url).await.unwrap_or(false) {
            debug!("Creating database {}", db_url);
            match sqlx::Any::create_database(db_url).await {
                Ok(_) => {
                    debug!("Success creating the database {}", db_url);
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        } else {
            return Err(GraphDbStartupError::DatabaseAlreadyCreated {
                database_name: db_url.to_string(),
            });
        }

        Self::connect_graph_database(db_url, connection_options).await
    }

    /// Creates the database that will be storing the project if no database exists with the given url.
    /// Otherwise, simply connects to it
    /// Returns an in instance of a [`GraphDatabase`].
    ///
    /// ## Errors
    ///
    /// * [`GraphDbRuntimeError`] if an unknown error was uncountered when trying to create/connect to it
    pub async fn connect_create_graph_database(
        db_url: &str,
        connection_options: Option<SqlxLogLevels>,
    ) -> Result<Self, GraphDbStartupError> {
        // Install sqlite, postgre and mysql drivers
        sqlx::any::install_default_drivers();

        if !sqlx::Any::database_exists(db_url).await.unwrap_or(false) {
            // Creates the database if it didn't already exists
            Self::create_graph_database(db_url, connection_options).await
        } else {
            // Or simply connects to it
            Self::connect_graph_database(db_url, connection_options).await
        }
    }

    /// Connects to the given database.
    /// Returns an in instance of a [`GraphDatabase`].
    ///
    /// ## Errors
    ///
    /// Returns a:
    /// * [`GraphDbStartupError`] if something went wrong during the connection.
    pub async fn connect_graph_database(
        db_url: &str,
        connection_options: Option<SqlxLogLevels>,
    ) -> Result<Self, GraphDbStartupError> {
        let pool = {
            let res = match connection_options {
                Some(opt) => Self::connect_with_options(db_url, opt).await,
                None => sqlx::Pool::connect(db_url).await,
            };
            match res {
                Ok(p) => p,
                Err(e) => return Err(e.into()),
            }
        };
        Ok(GraphDatabase::<DB> {
            // obs: None,
            // _url: db_url.to_string(),
            pool,
        })
    }

    /// Simply uses the given pool as the connection to the database.
    pub async fn connect_pool(pool: Pool<DB>) -> Self {
        GraphDatabase::<DB> { pool }
    }

    /// Closes the connection with the database (and drops this [`GraphDatabase`] instance)
    pub async fn close_connection(self) {
        self.pool.close().await
    }

    /// Applies the given options to the pool *before* opening it.
    async fn connect_with_options(
        url: &str,
        log_levels: SqlxLogLevels,
    ) -> Result<Pool<DB>, sqlx::Error> {
        let mut connect_opt = PoolOptions::new();

        if let Some(_level) = log_levels.log_statement_level {
            // connect_opt = connect_opt..(level).log_statements(level);
            // todo!("FIX THIS !")
            // TODO: AHHH
        }
        if let Some(level) = log_levels.log_slow_statement_level {
            connect_opt = connect_opt
                .acquire_slow_level(level.0)
                .acquire_slow_threshold(level.1);
        }
        connect_opt.connect(url).await
    }
}

// ______________________ ADD TABLES ____________

impl<DB> GraphDatabase<DB>
where
    DB: Database + DbQuerySystem<DB>,
{
    /// Adds the canonical table to the database.
    pub async fn add_canonical_table(&mut self) -> Result<(), GraphDbRuntimeError> {
        self.add_table(
            CANONICAL_TABLE_NAME,
            PK_NAME,
            ColumnType::String {
                max_size: Some(SIGNATURE_MAX_SIZE),
                default_value: None,
            },
            DATASET_VALUE_NAME,
            ColumnType::Integer {
                default_value: None,
            },
        )
        .await
    }

    /// Adds a metadata table to the database that will be used to not recompute the [`FULL_TABLE_NAME`] table.
    ///
    /// ## Exemple of table
    /// ```text
    /// Metadata -> | table_name  | stopped_at |
    ///             |-------------|------------|
    ///             | InitDataset | 1500       |
    ///             | Euler       | 753        |
    ///             |            ...           |
    /// ```
    /// ## Exceptions
    /// Returns:
    /// * [`GraphDbRuntimeError`] if something went wrong with the query
    async fn _add_meta_data_table(&mut self) -> Result<(), GraphDbRuntimeError> {
        self.add_table(
            METADATA_TABLE_NAME,
            METADATA_PK_NAME,
            ColumnType::String {
                max_size: Some(TABLE_NAME_MAX_SIZE),
                default_value: None,
            },
            METADATA_VALUE_NAME,
            ColumnType::Integer {
                default_value: Some(0),
            },
        )
        .await
    }

    async fn _add_invariant_table(&mut self, inv: String) -> Result<(), GraphDbRuntimeError> {
        self.add_table(
            inv,
            PK_NAME,
            ColumnType::String {
                max_size: Some(TABLE_NAME_MAX_SIZE),
                default_value: None,
            },
            INVARIANT_COLUMN_NAME,
            ColumnType::Integer {
                default_value: None,
            },
        )
        .await
    }

    pub async fn add_table(
        &mut self,
        table_name: impl ToString,
        pk_name: impl ToString,
        pk_column_type: ColumnType,
        value_name: impl ToString,
        value_column_type: ColumnType,
    ) -> Result<(), GraphDbRuntimeError> {
        let query_str = DB::get_create_table_query(
            table_name,
            pk_name,
            pk_column_type,
            value_name,
            value_column_type,
        );

        let builder = QueryBuilder::<DB>::new(query_str);
        DB::execute_query_no_return(&self.pool, builder).await
    }
}

// ______________________ UTILS ______________________

// TODO: Remove useless import
impl<DB: Database + DbQuerySystem<DB>> GraphDatabase<DB>
where
    usize: sqlx::ColumnIndex<<DB as sqlx::Database>::Row>,
    String: sqlx::Encode<'static, DB>,
    String: sqlx::Decode<'static, DB>,
    String: sqlx::Type<DB>,
    i16: sqlx::Decode<'static, DB>,
    i16: sqlx::Type<DB>,
    (i16,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
    i64: sqlx::Decode<'static, DB>,
    i64: sqlx::Type<DB>,
    (i64,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
    (String,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
    (String, i16): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
{
    pub async fn get_all_table_names(&self) -> Result<Vec<String>, GraphDbRuntimeError> {
        let query_str = DB::get_all_tables_query();
        let builder = QueryBuilder::<DB>::new(query_str);
        let results: Vec<(String,)> = DB::execute_query_fetch_all(&self.pool, builder).await?;

        Ok(results
            .iter()
            .map(|(val_str,)| val_str.to_string())
            .collect())
    }

    /// Checks if the given table was added.=
    pub async fn is_table_added(
        &self,
        table_name: impl ToString,
    ) -> Result<bool, GraphDbRuntimeError> {
        let query_str = DB::get_is_table_present(table_name);
        let builder = QueryBuilder::<DB>::new(query_str);
        let res: (i16,) = DB::execute_query_fetch_one(&self.pool, builder).await?;

        Ok(res.0 == 1)
    }

    /// Gets the number values inside the given table.
    /// TODO: Add unit test
    pub async fn get_size_of_table(
        &self,
        table_name: impl ToString,
    ) -> Result<usize, GraphDbRuntimeError> {
        let query_str = DB::get_nb_rows_from_table(table_name);

        let res: (i64,) =
            DB::execute_query_fetch_one(&self.pool, QueryBuilder::<DB>::new(query_str)).await?;

        Ok(res.0 as usize)
    }

    async fn _for_each_row<'e, V>(
        &self,
        table_name: String,
        apply_fn: &mut dyn FnMut(V) -> Result<(), GraphDbRuntimeError>,
    ) -> Result<(), GraphDbRuntimeError>
    where
        V: for<'r> FromRow<'r, DB::Row> + Send + Unpin,
    {
        let query_str = DB::get_all_from_table(table_name);

        let mut builder = QueryBuilder::<DB>::new(query_str);

        // Execute query :
        let mut results = DB::execute_query_fetch(&self.pool, &mut builder);

        while let Some(result_row) = results.next().await {
            match result_row {
                Ok(row) => apply_fn(row)?,
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    /// Reads and returns all values contained inside the [`CANONICAL_TABLE_NAME`].
    ///
    /// # Warning
    /// Only use this method when using small database because everything will be stored and returned.
    pub async fn read_all_dataset(&self) -> Result<Vec<(String, i16)>, GraphDbRuntimeError> {
        let query_str = DB::get_all_from_table(CANONICAL_TABLE_NAME);

        let builder = QueryBuilder::new(query_str);

        // Execute query :
        let results: Vec<(String, i16)> = DB::execute_query_fetch_all(&self.pool, builder).await?;

        Ok(results)
    }

    // /// Pretty prints all table to the standart output.
    // pub async fn print_all_tables(&self) -> Result<(), GraphDbRuntimeError> {
    //     // Get all tables :
    //     let tables = self.get_all_table_names().await?;
    //     if tables.is_empty() {
    //         println!("No tables in database");
    //         return Ok(());
    //     }
    //     for table in tables {
    //         let mut table_query: Option<QueryTable> = None;

    //         self.for_each_row(table, &mut |row: DB::Row| {
    //             // Fetch all header first
    //             if table_query.is_none() {
    //                 //
    //                 let mut headers = vec![];
    //                 for col in row.columns() {
    //                     headers.push(col.name().to_string());
    //                 }
    //                 table_query = Some(QueryTable::new(
    //                     headers,
    //                     crate::utils::table_handler::QueryTableOptions::Partial {
    //                         first_rows_count: 10,
    //                         last_rows_count: 10,
    //                     },
    //                 ));
    //             }
    //             // Add line to query
    //             table_query.as_mut().expect("is some").push_line({
    //                 let mut row_vec: Vec<String> = vec![];

    //                 for (i, col) in row.columns().iter().enumerate() {
    //                     // match col.type_info()() {
    //                     //     sqlx::any::AnyTypeInfoKind::BigInt
    //                     //     | sqlx::any::AnyTypeInfoKind::Integer => {
    //                     //         let value: i64 = row.get(i);
    //                     //         row_vec.push(value.to_string());
    //                     //     }
    //                     //     _ => {
    //                     //         if let Ok(value) = row.try_get(i) {
    //                     //             row_vec.push(value);
    //                     //         } else {
    //                     //             row_vec.push("[?Cannot convert to string?]".to_string());
    //                     //         }
    //                     //     }
    //                     //     sqlx::any::AnyTypeInfoKind::Null => todo!(),
    //                     //     sqlx::any::AnyTypeInfoKind::Bool => todo!(),
    //                     //     sqlx::any::AnyTypeInfoKind::SmallInt => todo!(),
    //                     //     sqlx::any::AnyTypeInfoKind::Real => todo!(),
    //                     //     sqlx::any::AnyTypeInfoKind::Double => todo!(),
    //                     //     sqlx::any::AnyTypeInfoKind::Text => todo!(),
    //                     //     sqlx::any::AnyTypeInfoKind::Blob => todo!(),
    //                     // }
    //                 }

    //                 row_vec
    //             });
    //             Ok(())
    //         })
    //         .await?;
    //         if let Some(tab) = table_query {
    //             println!("{tab}");
    //         }
    //     }

    //     Ok(())
    // }

    /// Add all canonical signatures to the table [`CANONICAL_TABLE_NAME`] of the dabase.
    /// Will not crash if a signature was already added previously.
    pub async fn add_to_dataset(
        &mut self,
        reader: impl BufRead,
        batch_size: usize,
    ) -> Result<(), GraphDbRuntimeError> {
        // Add dataset table if not already created
        let res = self.add_canonical_table().await;
        match &res {
            Ok(_) | Err(GraphDbRuntimeError::TableAlreadyCreatedError(_)) => {}
            _ => res?,
        }

        let push_to_db = async |vec: Vec<String>, batch_size| -> Result<(), GraphDbRuntimeError> {
            // Get insert query
            let query_str = DB::get_insert_into_query(CANONICAL_TABLE_NAME, 2, batch_size);
            let safe_query = Self::create_safe_query(query_str, vec)?;
            DB::execute_query_no_return(&self.pool, safe_query).await?;
            Ok(())
        };

        let mut data_batch = vec![];

        for canonical_form in reader.lines().map_while(Result::ok) {
            let value = get_nb_vertices(&canonical_form);
            data_batch.push(canonical_form);
            data_batch.push(value.to_string());

            if data_batch.len() / 2 >= batch_size {
                push_to_db(data_batch, batch_size).await?;
                data_batch = vec![];
            }
        }
        if !data_batch.is_empty() {
            let data_left = data_batch.len() / 2;
            push_to_db(data_batch, data_left).await?;
        }

        Ok(())
    }

    // async fn _add_to_inv_table(
    //     &mut self,
    //     inv_name: &String,
    //     values: Vec<String>,
    // ) -> Result<(), GraphDbRuntimeError> {
    //     let query_str = T::get_insert_into_query(inv_name, 2, inv_name.len());
    //     let safe_query = create_safe_query(query_str, values)?;
    //     self.execute_query_no_return(safe_query).await?;
    //     Ok(())
    // }

    fn create_safe_query(
        query_str: String,
        arguments: Vec<impl ToString>,
    ) -> Result<sqlx::QueryBuilder<'static, DB>, GraphDbRuntimeError> {
        let mut builder = QueryBuilder::<DB>::new("");
        let mut arguments = arguments.iter();

        for c in query_str.chars() {
            if c == '?' {
                match arguments.next() {
                    Some(arg) => {
                        builder.push_bind::<String>(arg.to_string());
                    }
                    None => {
                        return Err(GraphDbRuntimeError::QueryCreationError(
                            builder.into_sql().to_string(),
                        ));
                    }
                }
            } else {
                builder.push(c);
            }
        }
        Ok(builder)
    }

    // pub async fn compute_executable(
    //     &mut self,
    //     executable: &InvariantsExecutable,
    //     batch_size: usize,
    // ) -> Result<(), GraphDbRuntimeError> {
    //     // Check if we can compute this invariant
    //     for dep in &executable. {

    //     }

    //     // let query_str =
    //     // if let Some(dep) = &executable.dependencies {
    //     //     // Get dependencies
    //     //     let query_str = T::get_join_table_query(dep.clone(), INVARIANT_COLUMN_NAME);

    //     //     //
    //     // }

    //     // let mut batch_to_store = vec![];
    //     // executable.execute_invariant(&mut async || {
    //     //     Some(Ok("test".to_string()))
    //     // }, &mut async |val| {
    //     //     batch_to_store.push(val);
    //     //     if batch_to_store.len() >= batch_size {
    //     //         self.add_to_inv_table(inv_name, values)
    //     //         batch_to_store = vec![];
    //     //     }
    //     // }, batch_size);

    //     todo!()
    // }
}

fn get_nb_vertices(signature: &String) -> usize {
    let signature_byte = signature.as_bytes();
    // Check wether it is the extended format or not
    if signature_byte[0] != b'~' {
        (signature_byte[0] as usize) - 63
    }
    // Extended format
    else {
        (((signature_byte[1] - 63) as usize) << 18)
            | (((signature_byte[2] - 63) as usize) << 12)
            | (((signature_byte[3] - 63) as usize) << 6)
            | (((signature_byte[4] - 63) as usize) << 3)
    }
}
