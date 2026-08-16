use std::io::BufRead;
use std::{fmt::Debug, time::Duration};

use log::debug;
use sqlx::{Column, Row};
use sqlx::{Database, FromRow, Pool, QueryBuilder, migrate::MigrateDatabase, pool::PoolOptions};
use tokio_stream::{Stream, StreamExt};

use crate::data_handler::data_types::{ConstantValue, ValueType};
use crate::data_handler::module::{AsyncModuleInput, AsyncModuleOutput, Module, TypedArg};
use crate::data_handler::rel_graph::{FnArg, FnRef};
use crate::database_handler::{DbQuerySystem, GraphDbRuntimeError, *};
use crate::parser::parsed_expression::{Comparison, MathExpression};
use crate::utils::SaveOutput;
use crate::utils::subject::Observer;
use crate::utils::table_handler::{QueryTable, QueryTableOptions};

/// Used to change the logging settings of a sqlx connection.
/// This is because by default, all debug options will be shown and will fill the logger really quickly.
/// It is really recommended to set them to at least [`log::LevelFilter::Info`].
pub struct SqlxLogLevels {
    pub log_slow_statement_level: Option<(log::LevelFilter, Duration)>,
}

/// Trait used to encapsulate other traits a database should implement.
pub trait GraphDb: Database + DbQuerySystem<Self> + Send + MigrateDatabase {}

#[derive(Debug)]
/// Represents a graph database.
/// As long as it is not dropped, the connection to the related database will be up.
/// Can be cloned cheaply (since it will just clone the associated [`Pool`]).
pub struct GraphDatabase<DB: GraphDb> {
    pool: Pool<DB>,
}

// Note: We are not using derive since we don't have to force the DB to be clonable, only the pools.
impl<DB: GraphDb> Clone for GraphDatabase<DB> {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}

/// The GraphDatabase trait is used to facilitate the communication with databases for the user.
impl<DB: GraphDb> GraphDatabase<DB> {
    /// Creates the database that will be storing the project.
    /// Returns an in instance of a [GraphDatabase].
    ///
    /// ## Errors
    ///
    /// * [`GraphDbStartupError::DatabaseAlreadyCreated`] if the database was already created
    /// * [`GraphDbStartupError::DatabaseError`] if an unknown error was uncountered when trying to create it
    pub async fn create_graph_database(
        db_url: impl ToString,
        connection_options: Option<SqlxLogLevels>,
    ) -> Result<Self, GraphDbStartupError> {
        let db_url = db_url.to_string();
        // Install sqlite, postgre and mysql drivers
        // sqlx::any::install_default_drivers();

        // Creates the database if it didn't already exists
        if !DB::database_exists(&db_url).await.unwrap_or(false) {
            debug!("Creating database {}", db_url);
            match DB::create_database(&db_url).await {
                Ok(_) => {
                    debug!("Success creating the database {}", db_url);
                }
                Err(e) => {
                    return Err(DB::translate_startup_error(e));
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
    /// Otherwise, simply connects to it.
    /// Returns an in instance of a [`GraphDatabase`].
    ///
    /// ## Errors
    ///
    /// * [`GraphDbStartupError`] if an unknown error was uncountered when trying to create/connect to it
    pub async fn connect_create_graph_database(
        db_url: impl ToString,
        connection_options: Option<SqlxLogLevels>,
    ) -> Result<Self, GraphDbStartupError> {
        // Install sqlite, postgre and mysql drivers
        // sqlx::any::install_default_drivers();

        if !DB::database_exists(&db_url.to_string())
            .await
            .unwrap_or(false)
        {
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
        db_url: impl ToString,
        connection_options: Option<SqlxLogLevels>,
    ) -> Result<Self, GraphDbStartupError> {
        let pool = {
            let res = Self::connect_with_options(db_url, connection_options).await;
            match res {
                Ok(p) => p,
                Err(e) => return Err(DB::translate_startup_error(e)),
            }
        };

        Ok(GraphDatabase::<DB> { pool })
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
        url: impl ToString,
        log_levels: Option<SqlxLogLevels>,
    ) -> Result<Pool<DB>, sqlx::Error> {
        let mut connect_opt = PoolOptions::new().max_connections(20);

        if let Some(log_lvl) = log_levels
            && let Some(level) = log_lvl.log_slow_statement_level
        {
            connect_opt = connect_opt
                .acquire_slow_level(level.0)
                .acquire_slow_threshold(level.1);
        }
        connect_opt.connect(&url.to_string()).await
    }
}

// ______________________ ADD TABLES ____________

impl<DB: GraphDb> GraphDatabase<DB> {
    /// Adds the canonical table to the database.
    async fn add_canonical_table(&mut self) -> Result<(), GraphDbRuntimeError> {
        self.add_table(
            CANONICAL_TABLE_NAME,
            &[TypedArg {
                name: PK_NAME.to_string(),
                data_type: ValueType::Graph,
            }],
        )
        .await
    }

    /// Adds the vertices table to the database.
    async fn add_vertices_table(&mut self) -> Result<(), GraphDbRuntimeError> {
        self.add_table(
            VERTICES_TABLE_NAME,
            &[
                TypedArg {
                    name: PK_NAME.to_string(),
                    data_type: ValueType::Graph,
                },
                TypedArg {
                    name: FUNCTION_OUTPUT_COL_NAME.to_string(),
                    data_type: ValueType::Numeric,
                },
            ],
        )
        .await
    }

    async fn add_module_table(&mut self, module: &Module) -> Result<(), GraphDbRuntimeError> {
        let mut columns = module.args.clone();
        columns.push(TypedArg {
            name: FUNCTION_OUTPUT_COL_NAME.to_string(),
            data_type: module.output.clone(),
        });

        self.add_table(&module.fn_name, &columns).await
    }

    pub async fn add_table(
        &mut self,
        table_name: impl ToString,
        columns: &[TypedArg],
    ) -> Result<(), GraphDbRuntimeError> {
        let query_str = DB::get_create_table_query(table_name, columns);

        let builder = QueryBuilder::<DB>::new(query_str);
        DB::execute_query_no_return(&self.pool, builder).await
    }
}

// ______________________ UTILS ______________________

struct InputFn<'e, DB: GraphDb> {
    db: GraphDatabase<DB>,
    fetch: std::pin::Pin<
        Box<dyn Stream<Item = Result<<DB as Database>::Row, sqlx::Error>> + Send + 'e>,
    >,
}

impl<'e, DB: GraphDb> AsyncModuleInput<GraphDbRuntimeError> for InputFn<'e, DB>
where
    // Allow column indexing using usize
    usize: Send + Unpin + sqlx::ColumnIndex<DB::Row>,
    // Allow decoding/encoding
    String: sqlx::Encode<'static, DB>,
    f64: sqlx::Encode<'static, DB>,
    for<'a> String: sqlx::Decode<'a, DB>,
    for<'a> i64: sqlx::Decode<'a, DB>,
    for<'a> f64: sqlx::Decode<'a, DB>,
    for<'a> f32: sqlx::Decode<'a, DB>,
    for<'a> i32: sqlx::Decode<'a, DB>,
    // Type of values
    String: sqlx::Type<DB>,
    i64: sqlx::Type<DB>,
    f64: sqlx::Type<DB>,
    f32: sqlx::Type<DB>,
    // Return values
    (i32,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
{
    async fn call(&mut self) -> Option<Result<Vec<String>, GraphDbRuntimeError>> {
        let res: Result<<DB as Database>::Row, sqlx::Error> = self.fetch.next().await?;
        match res {
            Ok(row) => Some(Ok(self.db.read_row_val(&row))),
            Err(e) => Some(Err(DB::translate_runtime_error(e))),
        }
    }
}

struct OutputFn<'b, 'o, DB: GraphDb> {
    db: GraphDatabase<DB>,
    results: &'b mut Vec<Vec<ConstantValue>>,
    count: &'b mut usize,
    optional_obs: &'b mut Option<&'o mut dyn Observer>, // 'o lifetime for the observer itself
    batch_size: usize,
    module: Module,
}
impl<'b, 'o, DB: GraphDb> AsyncModuleOutput<GraphDbRuntimeError> for OutputFn<'b, 'o, DB>
where
    // Allow column indexing using usize
    usize: Send + Unpin + sqlx::ColumnIndex<DB::Row>,
    // Allow decoding/encoding
    String: sqlx::Encode<'static, DB>,
    f64: sqlx::Encode<'static, DB>,
    for<'a> String: sqlx::Decode<'a, DB>,
    for<'a> i64: sqlx::Decode<'a, DB>,
    for<'a> f64: sqlx::Decode<'a, DB>,
    for<'a> f32: sqlx::Decode<'a, DB>,
    for<'a> i32: sqlx::Decode<'a, DB>,
    // Type of values
    String: sqlx::Type<DB>,
    i64: sqlx::Type<DB>,
    f64: sqlx::Type<DB>,
    f32: sqlx::Type<DB>,
    // Return values
    (i32,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
{
    async fn call(&mut self, values: Vec<String>) -> Result<(), GraphDbRuntimeError> {
        // Received : arg_0, arg_1, ..., arg_{n-1}, output

        let mut received_values = Vec::with_capacity(values.len());

        for (i, arg) in values.iter().enumerate().take(values.len() - 1) {
            let arg_type = self
                .module
                .args
                .get(i)
                .expect("same number of args as returned values");

            received_values.push(arg_type.data_type.translate_into_const(arg)?);
        }
        // The last received value is always the result:
        received_values.push(
            self.module
                .output
                .translate_into_const(values.last().expect("one value present"))?,
        );

        self.results.push(received_values);

        // Notify obs something happened
        if let Some(obs) = &mut self.optional_obs {
            obs.notify_tick();
        }

        *self.count += 1;

        if *self.count >= self.batch_size {
            self.db.push_batch(self.results, &self.module).await?;
            if let Some(obs) = &mut self.optional_obs {
                obs.notify_data_pushed(self.batch_size as u64);
            }
            *self.count = 0;
        }
        Ok(())
    }
}

impl<DB: GraphDb> GraphDatabase<DB>
where
    // Allow column indexing using usize
    usize: Send + Unpin + sqlx::ColumnIndex<DB::Row>,
    // Allow decoding/encoding
    String: sqlx::Encode<'static, DB>,
    f64: sqlx::Encode<'static, DB>,
    for<'a> String: sqlx::Decode<'a, DB>,
    for<'a> i64: sqlx::Decode<'a, DB>,
    for<'a> f64: sqlx::Decode<'a, DB>,
    for<'a> f32: sqlx::Decode<'a, DB>,
    for<'a> i32: sqlx::Decode<'a, DB>,
    // Type of values
    String: sqlx::Type<DB>,
    i64: sqlx::Type<DB>,
    f64: sqlx::Type<DB>,
    f32: sqlx::Type<DB>,
    // Return values
    (i32,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
{
    /// Returns all the table name stored inside the database
    pub async fn get_all_table_names(&self) -> Result<Vec<String>, GraphDbRuntimeError> {
        let query_str = DB::get_all_tables_query();
        let mut builder = QueryBuilder::<DB>::new(query_str);
        let mut fetch_handle = DB::execute_query_fetch::<(String,)>(&self.pool, &mut builder);

        let mut results: Vec<String> = vec![];
        while let Some(res_value) = fetch_handle.next().await {
            match res_value {
                Ok(res) => results.push(res.0),
                Err(e) => return Err(DB::translate_runtime_error(e)),
            }
        }
        Ok(results)
    }

    /// Checks if the given table was added.
    pub async fn is_table_added(
        &self,
        table_name: impl ToString,
    ) -> Result<bool, GraphDbRuntimeError> {
        let query_str = DB::get_is_table_present(table_name);
        let builder = QueryBuilder::<DB>::new(query_str);
        let res: (i32,) = DB::execute_query_fetch_one(&self.pool, builder).await?;
        Ok(res.0 == 1)
    }

    /// Gets the number values stored inside the given table.
    pub async fn get_size_of_table(
        &self,
        table_name: impl ToString,
    ) -> Result<usize, GraphDbRuntimeError> {
        let query_str =
            SqlSelectQuery::select_count_all_from_table(table_name.to_string()).to_sql::<DB>();
        // let query_str = DB::get_nb_rows_from_table(table_name);

        let res: (i64,) =
            DB::execute_query_fetch_one(&self.pool, QueryBuilder::<DB>::new(query_str)).await?;

        Ok(res.0 as usize)
    }

    /// Reads and returns all values contained inside the [`CANONICAL_TABLE_NAME`].
    ///
    /// # Warning
    /// Only use this method when using small database because everything will be stored and returned.
    pub async fn read_all_dataset(&self) -> Result<Vec<String>, GraphDbRuntimeError> {
        let query_str =
            SqlSelectQuery::select_all_from_table(CANONICAL_TABLE_NAME.to_string()).to_sql::<DB>();

        let builder = QueryBuilder::new(query_str);
        // Execute query :
        let results: Vec<(String,)> = DB::execute_query_fetch_all(&self.pool, builder).await?;

        Ok(results.iter().map(|(sign,)| sign.to_string()).collect())
    }
    /// Reads and returns all values contained inside the given table.
    ///
    /// # Warning
    /// Only use this method when using small database because everything will be stored and returned.
    pub async fn read_all_table(
        &self,
        table_name: impl ToString,
    ) -> Result<Vec<(String, f64)>, GraphDbRuntimeError> {
        let query_str =
            SqlSelectQuery::select_all_from_table(table_name.to_string()).to_sql::<DB>();

        let builder = QueryBuilder::new(query_str);

        // Execute query :
        let results: Vec<(String, f64)> = DB::execute_query_fetch_all(&self.pool, builder).await?;

        Ok(results)
    }

    /// Pretty prints all table to the standart output.
    pub async fn print_all_tables(
        &self,
        with_id: bool,
        table_option: QueryTableOptions,
        to_latex: bool,
    ) -> Result<(), GraphDbRuntimeError> {
        // Get all tables :
        let tables = self.get_all_table_names().await?;
        if tables.is_empty() {
            println!("No tables in database");
            return Ok(());
        }
        for table in tables {
            let mut query_table = QueryTable::new_no_header(with_id, table_option.clone());
            self.fetch_all_row_query(
                &SqlSelectQuery::select_all_from_table(table),
                &mut query_table,
            )
            .await?;

            println!(
                "{}",
                if to_latex {
                    query_table.to_latex()
                } else {
                    query_table.to_string()
                }
            );
        }

        Ok(())
    }

    fn read_row_val(&self, row: &DB::Row) -> Vec<String> {
        Self::read_row_values(row)
    }

    fn read_row_values(row: &DB::Row) -> Vec<String> {
        let mut row_vec: Vec<String> = vec![];

        for col_i in 0..row.columns().len() {
            row_vec.push(Self::read_row_col_value(row, col_i));
        }

        row_vec
    }

    fn read_row_col_value(row: &DB::Row, col_index: usize) -> String {
        // let col = row.column(col_index);
        // let col_type = col.type_info().name();
        // // Float4 & Varchar are for postgre, the rest is for sqlite
        // if col_type == "INTEGER"  || col_type == "NULL" {
        //     let value = row.get::<i64, usize>(col_index);
        //     value.to_string()
        // } else if col_type == "REAL" || col_type == "FLOAT8"  {
        //     let value = row.get::<f64, usize>(col_index);
        //     value.to_string()
        // } else if col_type == "FLOAT4" {
        //     let value = row.get::<f32, usize>(col_index);
        //     value.to_string()
        // } else if col_type == "TEXT" || col_type == "VARCHAR"  {
        //     row.get::<String, usize>(col_index)
        // } else {
        //     "No string value".to_string()
        // }
        if let Ok(v) = row.try_get::<i64, _>(col_index) {
            return v.to_string();
        }

        if let Ok(v) = row.try_get::<f64, _>(col_index) {
            return v.to_string();
        }

        if let Ok(v) = row.try_get::<String, _>(col_index) {
            return v;
        }

        "No string value".to_string()
    }

    /// Removes the dataset and the related vertices table from the database.
    /// Does not return an error if no dataset were present.
    pub async fn remove_dataset(&mut self) -> Result<(), GraphDbRuntimeError> {
        if self.is_table_added(CANONICAL_TABLE_NAME).await? {
            let query = DB::get_delete_table_query(CANONICAL_TABLE_NAME);
            DB::execute_query_no_return(&self.pool, QueryBuilder::new(query)).await?;
        }

        if self.is_table_added(VERTICES_TABLE_NAME).await? {
            let query = DB::get_delete_table_query(VERTICES_TABLE_NAME);
            DB::execute_query_no_return(&self.pool, QueryBuilder::new(query)).await?;
        }
        Ok(())
    }

    async fn remove_tables(
        &mut self,
        table_names: &[String],
        restrict_mode: bool,
    ) -> Result<(), GraphDbRuntimeError> {
        for table in table_names {
            self.remove_table(table, restrict_mode).await?;
        }

        Ok(())
    }

    /// Removes the given table from the dataset **except** the dataset (and vertices) table.
    pub async fn remove_table(
        &mut self,
        table_name: impl ToString,
        restrict_mode: bool,
    ) -> Result<(), GraphDbRuntimeError> {
        let table = table_name.to_string().to_lowercase();
        if restrict_mode && (table == CANONICAL_TABLE_NAME || table == VERTICES_TABLE_NAME) {
            return Err(GraphDbRuntimeError::ForbiddenActionError {
                action: format!("Tried to remove the table \"{table}\" which is not allowed."),
            });
        }

        DB::execute_query_no_return(
            &self.pool,
            QueryBuilder::new(DB::get_delete_table_query(table_name)),
        )
        .await
    }

    /// Removes all tables from the dataset **except** the dataset (and vertices) table.
    pub async fn clear_invariants(
        &mut self,
        restrict_mode: bool,
    ) -> Result<(), GraphDbRuntimeError> {
        let mut table_names = Self::get_all_table_names(self).await?;

        table_names.retain(|name| name != CANONICAL_TABLE_NAME && name != VERTICES_TABLE_NAME);

        self.remove_tables(&table_names, restrict_mode).await
    }

    /// Removes all tables from the database, including the dataset (and vertices) table.
    pub async fn clear_database(&mut self, restrict_mode: bool) -> Result<(), GraphDbRuntimeError> {
        let table_names = Self::get_all_table_names(self).await?;

        self.remove_tables(&table_names, restrict_mode).await
    }

    /// Add all canonical signatures to the table [`CANONICAL_TABLE_NAME`] of the dabase.
    /// Will not crash if a signature was already added previously.
    /// If provided, the observer will be notified of every data pushed to the dataset and will tick after reading each signature.
    pub async fn add_to_dataset(
        &mut self,
        reader: impl BufRead,
        batch_size: usize,
        mut optional_obs: Option<&mut dyn Observer>,
    ) -> Result<(), GraphDbRuntimeError> {
        // Add dataset table if not already created
        let res = self.add_canonical_table().await;
        match &res {
            Ok(_) | Err(GraphDbRuntimeError::TableAlreadyCreatedError(_)) => {}
            _ => res?,
        }
        // Add vertices table if not already created
        let res = self.add_vertices_table().await;
        match &res {
            Ok(_) | Err(GraphDbRuntimeError::TableAlreadyCreatedError(_)) => {}
            _ => res?,
        }

        let push_to_db =
            async |signatures: &Vec<String>, batch_size| -> Result<(), GraphDbRuntimeError> {
                // Insert to dataset query
                let query_str = DB::get_insert_into_query(CANONICAL_TABLE_NAME, 1, batch_size);
                let safe_query = Self::create_safe_signature_query(query_str, signatures)?;
                DB::execute_query_no_return(&self.pool, safe_query).await
            };

        let push_to_vertices = async |values: &Vec<Vec<ConstantValue>>,
                                      batch_size|
               -> Result<(), GraphDbRuntimeError> {
            // Insert to vertices table query
            let query_str = DB::get_insert_into_query(VERTICES_TABLE_NAME, 2, batch_size);
            let safe_query = Self::create_safe_insert_query(query_str, 2, values)?;
            DB::execute_query_no_return(&self.pool, safe_query).await
        };

        let mut signature_batch = Vec::new();
        let mut data_batch = Vec::new();

        for canonical_form in reader.lines().map_while(Result::ok) {
            // Check input and get value
            let value = get_nb_vertices(&canonical_form)?;
            data_batch.push(vec![
                ConstantValue::String(canonical_form.clone()),
                ConstantValue::Numeric((value as f64).into()),
            ]);

            signature_batch.push(canonical_form);

            if data_batch.len() >= batch_size {
                // push collected data to database
                push_to_db(&signature_batch, batch_size).await?;
                push_to_vertices(&data_batch, batch_size).await?;
                if let Some(obs) = &mut optional_obs {
                    obs.notify_data_pushed(batch_size.try_into().expect("val to be u64"));
                }
                // Clear has no effect on batch capacity
                data_batch.clear();
                signature_batch.clear();
            }

            if let Some(obs) = &mut optional_obs {
                obs.notify_tick();
            }
        }
        // If any is still cached, store it in the database
        if !data_batch.is_empty() {
            push_to_db(&signature_batch, data_batch.len()).await?;
            push_to_vertices(&data_batch, data_batch.len()).await?;

            if let Some(obs) = &mut optional_obs {
                obs.notify_data_pushed(data_batch.len().try_into().expect("val to be u64"));
            }
        }

        Ok(())
    }

    async fn add_to_function_table(
        &mut self,
        function_name: impl ToString,
        nb_args: usize,
        results_to_store: &mut [Vec<ConstantValue>],
    ) -> Result<(), GraphDbRuntimeError> {
        let inv_name = function_name.to_string();

        let query_str = DB::get_insert_into_query(&inv_name, nb_args, results_to_store.len());

        let safe_query = Self::create_safe_insert_query(query_str, nb_args, results_to_store)?;
        DB::execute_query_no_return(&self.pool, safe_query).await
    }

    fn create_safe_insert_query(
        query_str: String,
        nb_cols: usize,
        results_to_store: &[Vec<ConstantValue>],
    ) -> Result<sqlx::QueryBuilder<'static, DB>, GraphDbRuntimeError> {
        let mut builder = QueryBuilder::<DB>::new("");
        let mut value_id = 0;
        let mut col_id = 0;

        for c in query_str.chars() {
            if c == '?' {
                let value = &results_to_store[value_id][col_id];
                match &value {
                    ConstantValue::Numeric(nb) => {
                        builder.push_bind::<f64>(nb.0);
                    }
                    ConstantValue::Identifier(str) | ConstantValue::String(str) => {
                        builder.push_bind::<String>(str.clone());
                    }
                    // Boolean values depend on the database
                    ConstantValue::Bool(_) => {
                        builder.push_bind::<String>(DB::translate_constant(value));
                    }
                }
                col_id += 1;
                if col_id == nb_cols {
                    col_id = 0;
                    value_id += 1;
                }
            } else {
                builder.push(c);
            }
        }
        Ok(builder)
    }

    fn create_safe_signature_query(
        query_str: String,
        signatures: &[impl ToString],
    ) -> Result<sqlx::QueryBuilder<'static, DB>, GraphDbRuntimeError> {
        let mut builder = QueryBuilder::<DB>::new("");
        let mut signatures = signatures.iter();

        for c in query_str.chars() {
            if c == '?' {
                match signatures.next() {
                    Some(sign) => {
                        builder.push_bind::<String>(sign.to_string());
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

    /// Computes an executable and store its results in one or more tables.
    ///
    /// # Args :
    /// * When provided, only signatures contained inside the result of this query will be inputed to the invariant, further restricting the input space.
    ///     * The [`PK_NAME`] column must be the only one present in this query, else an error might happen.
    /// * If provided, the given observer will be ticked for every data received and notified of the data pushed.
    #[allow(clippy::too_many_arguments)]
    pub async fn compute_module(
        &mut self,
        fn_ref: &FnRef,
        module: &Module,
        args: &[MathExpression<FnArg>],
        dataset_to_use: Option<SqlSelectQuery>,
        join_list: Vec<SqlJoin>,
        batch_size: usize,
        mut optional_obs: Option<&mut dyn Observer>,
    ) -> Result<(), GraphDbRuntimeError> {
        // Check if the dataset was at least initialised first
        if !&self.is_table_added(CANONICAL_TABLE_NAME).await? {
            return Err(GraphDbRuntimeError::DatasetNotInitialisedError);
        }

        // Take the minimum between the two batch sizes.
        let batch_size = match module.batch_size {
            Some(module_batch) => module_batch.min(batch_size),
            None => batch_size,
        };

        // TODO: Check if there are only constants (since this will not require the need for a join...)

        // Using the given join list, we can restrict the dataset.

        let mut input_selection = match dataset_to_use {
            Some(dataset) => {
                // Rename it to dataset for it to act as the classic table.
                let dataset_table = SqlTableSelection::new_rename(dataset, CANONICAL_TABLE_NAME);
                SqlSelectQuery::select_columns_from_table(args.to_vec(), dataset_table)
            }
            None => SqlSelectQuery::select_columns_from_table(args.to_vec(), CANONICAL_TABLE_NAME),
        };

        // We do not want to re send a row multiple times (useful for functions that do not depend on a graph signature like d(n,m) for example)
        input_selection.set_distinct_values(true);

        for join in join_list {
            input_selection.add_join(join);
        }

        /* if the table already exists, adds a condition so that only gets values not present in it */
        {
            if self.is_table_added(&module.fn_name).await? {
                let mut not_exist_selection =
                    SqlSelectQuery::select_all_from_table(module.fn_name.to_string());

                for (i, arg) in module.args.iter().enumerate() {
                    not_exist_selection.add_and(Comparison::Equal(
                        FnArg::Constant(ConstantValue::Identifier(arg.name.clone())).into(),
                        args[i].clone(),
                        None,
                    ));
                }
                input_selection.where_clause =
                    Some(SqlWhereClause::not_exists(not_exist_selection));
            }
        }

        // Build the final query
        let query_str = input_selection.to_sql::<DB>();
        // Set the capacity of the vectors to save time (since we know their maximum sizes)
        let mut args_to_store: Vec<Vec<ConstantValue>> = Vec::with_capacity(batch_size);

        let mut count = 0;

        {
            let fetch_handle: std::pin::Pin<
                Box<dyn Stream<Item = Result<<DB as Database>::Row, sqlx::Error>> + Send>,
            > = DB::execute_query_fetch_sql_rows(&self.pool, sqlx::query(&query_str));

            let input = InputFn::<'_, DB> {
                db: self.clone(),
                fetch: fetch_handle,
            };

            let output = OutputFn {
                db: self.clone(),
                results: &mut args_to_store,
                count: &mut count,
                batch_size,
                module: module.clone(),
                optional_obs: &mut optional_obs,
            };

            module.execute(fn_ref, input, output, batch_size).await?;
        }
        if count != 0 {
            self.push_batch(&mut args_to_store, module).await?;

            if let Some(obs) = &mut optional_obs {
                obs.notify_data_pushed(count as u64);
            }
        }
        Ok(())
    }

    async fn push_batch(
        &mut self,
        results: &mut Vec<Vec<ConstantValue>>,
        module: &Module,
    ) -> Result<(), GraphDbRuntimeError> {
        // Push data to related function table
        if !self.is_table_added(&module.fn_name).await? {
            self.add_module_table(module).await?
        }
        self.add_to_function_table(&module.fn_name, module.args.len() + 1, results)
            .await?;
        results.clear();
        Ok(())
    }

    /// Fetches and stores all rows from the given raw sql query as strings.
    pub async fn fetch_all_rows_raw_sql<O>(
        &self,
        query: impl ToString,
        output: &mut O,
    ) -> Result<(), GraphDbRuntimeError>
    where
        O: SaveOutput,
    {
        let mut added_headers = false;
        let query = query.to_string();
        // Execute query :
        let mut results = DB::execute_query_fetch_sql_rows(&self.pool, sqlx::query(&query));
        // Add each returned row to the table
        while let Some(result_row) = results.next().await {
            let row = match result_row {
                Ok(row) => row,
                Err(e) => return Err(DB::translate_runtime_error(e)),
            };
            // Fetch all header first
            if !added_headers {
                added_headers = true;
                let mut headers = vec![];
                for col in row.columns() {
                    headers.push(col.name().to_string());
                }
                output.push_line(headers);
            }
            output.push_line(Self::read_row_values(&row));
        }
        Ok(())
    }

    /// Fetches and stores all rows from the given query.
    pub async fn fetch_all_row_query<O>(
        &self,
        query: &SqlSelectQuery,
        output: &mut O,
    ) -> Result<(), GraphDbRuntimeError>
    where
        O: SaveOutput,
    {
        self.fetch_all_rows_raw_sql(query.to_sql::<DB>(), output)
            .await
    }
}

fn get_nb_vertices(signature: &str) -> Result<usize, GraphDbRuntimeError> {
    let signature = signature.trim(); // remove any space

    let signature_byte = signature.as_bytes();
    // Check signature validity :
    if signature_byte.is_empty() || !signature_byte.iter().all(|b| *b >= 63 && *b <= 126) {
        return Err(GraphDbRuntimeError::InvalidSignature(signature.to_owned()));
    }
    // Check wether it is the extended format or not
    if signature_byte[0] != b'~' {
        Ok((signature_byte[0] as usize) - 63)
    }
    // Extended format
    else {
        if signature_byte.len() < 5 {
            return Err(GraphDbRuntimeError::InvalidSignature(signature.to_string()));
        }
        Ok((((signature_byte[1] - 63) as usize) << 18)
            | (((signature_byte[2] - 63) as usize) << 12)
            | (((signature_byte[3] - 63) as usize) << 6)
            | (((signature_byte[4] - 63) as usize) << 3))
    }
}
