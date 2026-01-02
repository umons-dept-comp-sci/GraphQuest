use std::io::BufRead;
use std::{fmt::Debug, time::Duration};

use log::debug;
use sqlx::{Column, Row, TypeInfo};
use sqlx::{Database, FromRow, Pool, QueryBuilder, migrate::MigrateDatabase, pool::PoolOptions};
use tokio_stream::{Stream, StreamExt};

use crate::data_handler::invariant_execs::{
    AsyncInvariantInput, AsyncInvariantOutput, InvariantsExecutable,
};
use crate::database_handler::{DbQuerySystem, GraphDbRuntimeError, *};
use crate::utils::SaveOutput;
use crate::utils::subject::Observer;
use crate::utils::table_handler::{QueryTable, QueryTableOptions};

/// Used to change the logging settings of a sqlx connection.
/// This is because by default, all debug options will be shown and will fill the logger really quickly.
/// It is really recommended to set them to at least [`log::LevelFilter::Info`].
pub struct SqlxLogLevels {
    pub log_slow_statement_level: Option<(log::LevelFilter, Duration)>,
}

#[derive(Debug)]
/// Represents a graph database.
/// As long as it is not dropped, the connection to the related database will be stay up.
/// Can be cloned cheaply (since it will just clone the associated [`Pool`]).
pub struct GraphDatabase<DB>
where
    DB: Database + DbQuerySystem<DB> + Send,
{
    pool: Pool<DB>,
}

// Note: We are not using derive since we don't have to force the DB to be clonable, only the pools.
impl<DB> Clone for GraphDatabase<DB>
where
    DB: Database + DbQuerySystem<DB>,
{
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}

/// The GraphDatabase trait is used to facilitate the communication with databases for the user.
impl<DB: Database + DbQuerySystem<DB>> GraphDatabase<DB>
where
    DB: MigrateDatabase,
{
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
            let res = match connection_options {
                Some(opt) => Self::connect_with_options(db_url, opt).await,
                None => sqlx::Pool::connect(&db_url.to_string()).await,
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
        url: impl ToString,
        log_levels: SqlxLogLevels,
    ) -> Result<Pool<DB>, sqlx::Error> {
        let mut connect_opt = PoolOptions::new();

        if let Some(level) = log_levels.log_slow_statement_level {
            connect_opt = connect_opt
                .acquire_slow_level(level.0)
                .acquire_slow_threshold(level.1);
        }
        connect_opt.connect(&url.to_string()).await
    }
}

// ______________________ ADD TABLES ____________

impl<DB> GraphDatabase<DB>
where
    DB: Database + DbQuerySystem<DB>,
{
    /// Adds the canonical table to the database.
    async fn add_canonical_table(&mut self) -> Result<(), GraphDbRuntimeError> {
        let query_str = DB::get_create_table_query(
            CANONICAL_TABLE_NAME,
            PK_NAME,
            ColumnType::String {
                max_size: Some(SIGNATURE_MAX_SIZE),
                default_value: None,
            },
        );

        let builder = QueryBuilder::<DB>::new(query_str);
        DB::execute_query_no_return(&self.pool, builder).await
    }

    /// Adds the vertices table to the database.
    async fn add_vertices_table(&mut self) -> Result<(), GraphDbRuntimeError> {
        self.add_invariant_table(VERTICES_TABLE_NAME.to_string())
            .await
    }

    async fn add_invariant_table(&mut self, inv: String) -> Result<(), GraphDbRuntimeError> {
        self.add_table(
            inv.to_string(),
            PK_NAME,
            ColumnType::String {
                max_size: Some(TABLE_NAME_MAX_SIZE),
                default_value: None,
            },
            inv, // Makes the join queries easier
            // We store data as strings in order to accept anything
            ColumnType::Float {
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
        let query_str = DB::get_create_table_value_query(
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

struct InputFn<'e, DB: Database + DbQuerySystem<DB>> {
    db: GraphDatabase<DB>,
    fetch: std::pin::Pin<
        Box<dyn Stream<Item = Result<<DB as Database>::Row, sqlx::Error>> + Send + 'e>,
    >,
}

impl<'e, DB: Database + DbQuerySystem<DB>> AsyncInvariantInput<GraphDbRuntimeError>
    for InputFn<'e, DB>
where
    DB: Send,
    DB: MigrateDatabase,
    // Allow column indexing using usize
    usize: Send + Unpin + sqlx::ColumnIndex<DB::Row>,
    // Allow decoding/encoding
    String: sqlx::Encode<'static, DB>,
    for<'a> String: sqlx::Decode<'a, DB>,
    for<'a> i64: sqlx::Decode<'a, DB>,
    for<'a> f64: sqlx::Decode<'a, DB>,
    // Type of values
    String: sqlx::Type<DB>,
    i64: sqlx::Type<DB>,
    f64: sqlx::Type<DB>,
    // Return values
    (i64,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
    (String,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
    (String, f64): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
{
    async fn call(&mut self) -> Option<Result<Vec<String>, GraphDbRuntimeError>> {
        let res: Result<<DB as Database>::Row, sqlx::Error> = self.fetch.next().await?;
        match res {
            Ok(row) => Some(Ok(self.db.read_row_val(&row))),
            Err(e) => Some(Err(e.into())),
        }
    }
}

struct OutputFn<'b, 'o, DB: Database + DbQuerySystem<DB>> {
    db: GraphDatabase<DB>,
    signatures: &'b mut Vec<String>,
    batch_to_store: &'b mut Vec<Vec<String>>,
    count: &'b mut usize,
    optional_obs: &'b mut Option<&'o mut dyn Observer>, // 'o lifetime for the observer itself
    batch_size: usize,
    invariant_exec: InvariantsExecutable,
}
impl<'b, 'o, DB: Database + DbQuerySystem<DB>> AsyncInvariantOutput<GraphDbRuntimeError>
    for OutputFn<'b, 'o, DB>
where
    DB: Send,
    DB: MigrateDatabase,
    // Allow column indexing using usize
    usize: Send + Unpin + sqlx::ColumnIndex<DB::Row>,
    // Allow decoding/encoding
    String: sqlx::Encode<'static, DB>,
    for<'a> String: sqlx::Decode<'a, DB>,
    for<'a> i64: sqlx::Decode<'a, DB>,
    for<'a> f64: sqlx::Decode<'a, DB>,
    // Type of values
    String: sqlx::Type<DB>,
    i64: sqlx::Type<DB>,
    f64: sqlx::Type<DB>,
    // Return values
    (i64,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
    (String,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
    (String, f64): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
{
    async fn call(&mut self, values: Vec<String>) -> Result<(), GraphDbRuntimeError> {
        let mut values = values.into_iter();
        self.signatures
            .push(values.next().expect("a signature as the first value"));

        // Received : inv_0, inv_1, inv_2, ..., inv_{n-1}
        for (i, inv_values) in values.enumerate() {
            // Push into the related storing vector
            self.batch_to_store
                .get_mut(i)
                .expect("correct index")
                .push(inv_values);
        }

        // Notify obs something happened
        if let Some(obs) = &mut self.optional_obs {
            obs.notify_tick();
        }

        *self.count += 1;

        if *self.count >= self.batch_size {
            self.db
                .push_batch(self.signatures, self.batch_to_store, &self.invariant_exec)
                .await?;
            if let Some(obs) = &mut self.optional_obs {
                obs.notify_data_pushed(self.batch_size as u64);
            }
            *self.count = 0;
        }
        Ok(())
    }
}

// TODO: Remove useless import
impl<DB: Database + DbQuerySystem<DB>> GraphDatabase<DB>
where
    DB: Send,
    DB: MigrateDatabase,
    // Allow column indexing using usize
    usize: Send + Unpin + sqlx::ColumnIndex<DB::Row>,
    // Allow decoding/encoding
    String: sqlx::Encode<'static, DB>,
    for<'a> String: sqlx::Decode<'a, DB>,
    for<'a> i64: sqlx::Decode<'a, DB>,
    for<'a> f64: sqlx::Decode<'a, DB>,
    // Type of values
    String: sqlx::Type<DB>,
    i64: sqlx::Type<DB>,
    f64: sqlx::Type<DB>,
    // Return values
    (i64,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
    (String,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
    (String, f64): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
{
    /// Returns all the table name stored inside the database
    pub async fn get_all_table_names(&self) -> Result<Vec<String>, GraphDbRuntimeError> {
        let query_str = DB::get_all_tables_query();
        let mut builder = QueryBuilder::<DB>::new(query_str);
        let mut fetch_handle = DB::execute_query_fetch::<(String,)>(&self.pool, &mut builder);

        let mut results: Vec<String> = vec![];
        while let Some(res_value) = fetch_handle.next().await {
            results.push(res_value?.0);
        }
        Ok(results)
    }

    /// Checks if the given table was added.=
    pub async fn is_table_added(
        &self,
        table_name: impl ToString,
    ) -> Result<bool, GraphDbRuntimeError> {
        let query_str = DB::get_is_table_present(table_name);
        let builder = QueryBuilder::<DB>::new(query_str);
        let res: (i64,) = DB::execute_query_fetch_one(&self.pool, builder).await?;

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
        table_option: QueryTableOptions,
    ) -> Result<(), GraphDbRuntimeError> {
        // Get all tables :
        let tables = self.get_all_table_names().await?;
        if tables.is_empty() {
            println!("No tables in database");
            return Ok(());
        }
        for table in tables {
            let mut query_table = QueryTable::new_no_header(table_option.clone());
            self.fetch_all_row_query(
                &SqlSelectQuery::select_all_from_table(table),
                &mut query_table,
            )
            .await?;

            println!("{query_table}");
        }

        Ok(())
    }

    pub fn read_row_val(&self, row: &DB::Row) -> Vec<String> {
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
        let col = row.column(col_index);
        let col_type = col.type_info().name();
        if col_type == "INTEGER" {
            let value = row.get::<i64, usize>(col_index);
            value.to_string()
        } else if col_type == "REAL" || col_type == "NULL" {
            let value = row.get::<f64, usize>(col_index);
            value.to_string()
        } else if col_type == "TEXT" {
            row.get::<String, usize>(col_index)
        } else {
            "No string value".to_string()
        }
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

    async fn remove_tables(&mut self, table_names: &[String]) -> Result<(), GraphDbRuntimeError> {
        for table in table_names {
            DB::execute_query_no_return(
                &self.pool,
                QueryBuilder::new(DB::get_delete_table_query(table)),
            )
            .await?;
        }

        Ok(())
    }

    /// Removes all tables from the dataset **except** the dataset (and vertices) table.
    pub async fn clear_invariants(&mut self) -> Result<(), GraphDbRuntimeError> {
        let mut table_names = Self::get_all_table_names(self).await?;

        table_names.retain(|name| name != CANONICAL_TABLE_NAME && name != VERTICES_TABLE_NAME);

        self.remove_tables(&table_names).await
    }

    /// Removes all tables from the dataset (and vertices) table.
    pub async fn clear_database(&mut self) -> Result<(), GraphDbRuntimeError> {
        let table_names = Self::get_all_table_names(self).await?;

        self.remove_tables(&table_names).await
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

        let push_to_db = async |signatures: &Vec<String>,
                                values: &Vec<String>,
                                batch_size|
               -> Result<(), GraphDbRuntimeError> {
            // Insert to dataset query
            let query_str = DB::get_insert_into_query(CANONICAL_TABLE_NAME, 1, batch_size);
            let safe_query = Self::create_safe_signature_query(query_str, signatures)?;
            DB::execute_query_no_return(&self.pool, safe_query).await?;

            // Insert to vertices table query
            let query_str = DB::get_insert_into_query(VERTICES_TABLE_NAME, 2, batch_size);
            let safe_query = Self::create_safe_insert_query(query_str, signatures, values)?;
            DB::execute_query_no_return(&self.pool, safe_query).await?;
            Ok(())
        };

        let mut signature_batch = vec![];
        let mut data_batch = vec![];

        for canonical_form in reader.lines().map_while(Result::ok) {
            // Check input and get value
            let value = get_nb_vertices(&canonical_form)?;
            signature_batch.push(canonical_form);
            data_batch.push(value.to_string());

            if data_batch.len() >= batch_size {
                // push collected data to database
                push_to_db(&signature_batch, &data_batch, batch_size).await?;
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
            push_to_db(&signature_batch, &data_batch, data_batch.len()).await?;

            if let Some(obs) = &mut optional_obs {
                obs.notify_data_pushed(data_batch.len().try_into().expect("val to be u64"));
            }
        }

        Ok(())
    }

    async fn add_to_inv_table(
        &mut self,
        inv_name: impl ToString,
        signatures: &[impl ToString],
        values: &[impl ToString],
    ) -> Result<(), GraphDbRuntimeError> {
        let inv_name = inv_name.to_string();

        // TODO: Combine signatures and values into one vector
        let query_str = DB::get_insert_into_query(&inv_name, 2, signatures.len());

        let safe_query = Self::create_safe_insert_query(query_str, signatures, values)?;
        DB::execute_query_no_return(&self.pool, safe_query).await
    }

    fn create_safe_insert_query(
        query_str: String,
        signatures: &[impl ToString],
        values: &[impl ToString],
    ) -> Result<sqlx::QueryBuilder<'static, DB>, GraphDbRuntimeError> {
        let mut builder = QueryBuilder::<DB>::new("");
        let mut signatures = signatures.iter();
        let mut values = values.iter();

        let mut added_signature = false;

        for c in query_str.chars() {
            if c == '?' {
                if added_signature {
                    match values.next() {
                        Some(value) => {
                            builder.push_bind::<String>(value.to_string());
                        }
                        None => {
                            return Err(GraphDbRuntimeError::QueryCreationError(
                                builder.into_sql().to_string(),
                            ));
                        }
                    }
                } else {
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
                }
                added_signature = !added_signature;
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
    ///     * The [`PK_NAME`] column must be present in this query, else an error might happen.
    /// * If provided, the given observer will be ticked for every data received and notified of the data pushed.
    pub async fn compute_executable(
        &mut self,
        executable: &InvariantsExecutable,
        add_query: Option<SqlSelectQuery>,
        batch_size: usize,
        mut optional_obs: Option<&mut dyn Observer>,
    ) -> Result<(), GraphDbRuntimeError> {
        // Check if the dataset was at least initialised first
        if !&self.is_table_added(CANONICAL_TABLE_NAME).await? {
            return Err(GraphDbRuntimeError::DatasetNotInitialisedError(
                executable.clone(),
            ));
        }

        // Check if we can compute this invariant
        // by checking that every dependency was added before
        if let Some(deps) = &executable.dependencies {
            for dep in deps {
                if !&self.is_table_added(dep).await? {
                    return Err(GraphDbRuntimeError::InvariantDependencyError(
                        executable.clone(),
                        dep.to_string(),
                    ));
                }
            }
        }

        /* Join dependencies if any */
        let mut join_query = {
            if let Some(dep) = &executable.dependencies
                && !dep.is_empty()
            {
                let mut dep = dep.clone();
                // Get dependencies
                SqlSelectQuery::select_column_from_table(
                    format!("{CANONICAL_TABLE_NAME}.*"),
                    SqlTableSelection {
                        selected_table: SqlSelectQuery::select_all_from_table(
                            SqlTableSelection::new_join(
                                dep[0].clone(),
                                dep.split_off(1),
                                PK_NAME,
                                None,
                            ),
                        )
                        .into(),
                        join_clause: None,
                        rename_as: Some(CANONICAL_TABLE_NAME.to_string()),
                    },
                )
            } else {
                SqlSelectQuery::select_column_from_table(
                    format!("{CANONICAL_TABLE_NAME}.*"),
                    CANONICAL_TABLE_NAME,
                )
            }
        };

        // If given, uses that additional querry to further restrict the input
        if let Some(select_query) = add_query {
            let additional_table = SqlTableSelection {
                selected_table: select_query.into(),
                join_clause: None,
                rename_as: Some(TEMPORARY_TABLE_NAME.to_string()),
            };
            join_query.add_table(additional_table);
            join_query.add_and(SqlComparison::Equal(
                ArgType::Identifier(format!("{CANONICAL_TABLE_NAME}.{PK_NAME}")),
                ArgType::Identifier(format!("{TEMPORARY_TABLE_NAME}.{PK_NAME}")),
            ));
        }

        /* if table already exists, add a condition so that only gets value not present in the invariant table will be returned */
        // Remember that since all invariants are computed together from an executable we can simply check for one and it will apply to all.
        {
            let first_inv = executable.invariant_names.last().expect("at least one val");
            if self.is_table_added(first_inv).await? {
                join_query.add_and(SqlCondition::not(SqlCondition::exists(SqlSelectQuery {
                    select: vec![PK_NAME.to_string()],
                    from: vec![first_inv.to_string().into()],
                    group_by: vec![],
                    where_clause: Some(
                        SqlComparison::Equal(
                            ArgType::Identifier(format!("{first_inv}.{PK_NAME}")),
                            ArgType::Identifier(format!("{CANONICAL_TABLE_NAME}.{PK_NAME}")),
                        )
                        .into(),
                    ),
                    limit: None,
                })));
            }
        }
        // build query
        let query_str = join_query.to_sql::<DB>();

        // Set the capacity of the vector to save time (since we know their sizes)
        let mut signatures: Vec<String> = Vec::with_capacity(batch_size);
        let mut batch_to_store: Vec<Vec<String>> =
            Vec::with_capacity(executable.invariant_names.len());
        (0..executable.invariant_names.len())
            .for_each(|_| batch_to_store.push(Vec::with_capacity(batch_size)));

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
                signatures: &mut signatures,
                batch_to_store: &mut batch_to_store,
                count: &mut count,
                batch_size,
                invariant_exec: executable.clone(),
                optional_obs: &mut optional_obs,
            };

            // executable.exec_inv(input).await;
            executable
                .execute_invariant(input, output, batch_size)
                .await?;
        }
        if count != 0 {
            let batch_len = batch_to_store[0].len(); // Saving that to notify *after* saving the data

            self.push_batch(&mut signatures, &mut batch_to_store, executable)
                .await?;

            if let Some(obs) = &mut optional_obs {
                obs.notify_data_pushed(batch_len as u64);
            }
        }
        Ok(())
    }

    async fn push_batch(
        &mut self,
        signatures: &mut Vec<String>,
        batch_to_store: &mut [Vec<String>],
        executable: &InvariantsExecutable,
    ) -> Result<(), GraphDbRuntimeError> {
        for (i, inv_values) in batch_to_store.iter_mut().enumerate() {
            // Check if table was created before
            if !self.is_table_added(&executable.invariant_names[i]).await? {
                self.add_invariant_table(executable.invariant_names[i].clone())
                    .await?
            }
            // Push data to related invariant table
            self.add_to_inv_table(&executable.invariant_names[i], signatures, inv_values)
                .await?;

            inv_values.clear(); // no affects on capacity
        }
        signatures.clear();
        Ok(())
    }

    /// Fetches and stores all rows from the given query inside a table as strings.
    pub async fn fetch_all_row_query<O>(
        &self,
        query: &SqlSelectQuery,
        output: &mut O,
    ) -> Result<(), GraphDbRuntimeError>
    where
        O: SaveOutput,
    {
        let mut added_headers = false;
        let query = query.to_sql::<DB>();
        // Execute query :
        let mut results = DB::execute_query_fetch_sql_rows(&self.pool, sqlx::query(&query));
        // Add each returned row to the table
        while let Some(result_row) = results.next().await {
            let row = result_row?;
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
