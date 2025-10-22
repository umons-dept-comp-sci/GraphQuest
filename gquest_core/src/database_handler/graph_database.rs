use std::{fmt::Debug, io::BufRead, marker::PhantomData, str::FromStr, time::Duration};

use log::debug;
use sqlx::{
    any::{AnyConnectOptions, AnyRow},
    migrate::MigrateDatabase,
    AnyPool, Column, ConnectOptions, Pool, QueryBuilder, Row,
};
use tokio_stream::StreamExt;

use crate::{
    database_handler::{database_error, DbQuerySystem, GraphDatabaseError, *},
    utils::table_handler::QueryTable,
};

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
pub struct GraphDatabase<T>
where
    T: DbQuerySystem,
{
    pool: Pool<sqlx::Any>,
    url: String,
    phantom_data: PhantomData<T>,
}

/// The GraphDatabase trait is used to facilitate the communication with databases for the user.
impl<T: DbQuerySystem> GraphDatabase<T> {
    /// Creates the database that will be storing the project.
    /// Returns an in instance of a [GraphDatabase].
    ///
    /// ## Errors
    ///
    /// * [GraphDatabaseError::DatabaseAlreadyCreated] if the database was already created
    /// * [GraphDatabaseError::DatabaseError] if an unknown error was uncountered when trying to create it
    pub async fn create_graph_database(
        db_url: &str,
        connection_options: Option<SqlxLogLevels>,
    ) -> Result<Self, GraphDatabaseError> {
        // Install sqlite, postgre and mysql drivers
        sqlx::any::install_default_drivers();

        // Creates the database if it didn't already exists
        if !sqlx::Any::database_exists(db_url).await.unwrap_or(false) {
            debug!("Creating database {}", db_url);
            match sqlx::Any::create_database(db_url).await {
                Ok(_) => {
                    debug!("Success creating the database {}", db_url);
                }
                Err(error) => {
                    return Err(GraphDatabaseError::DatabaseError {
                        database_name: db_url.to_string(),
                        reason: error.to_string(),
                    });
                }
            }
        } else {
            return Err(GraphDatabaseError::DatabaseAlreadyCreated {
                database_name: db_url.to_string(),
            });
        }

        Self::connect_graph_database(db_url, connection_options).await
    }

    /// Creates the database that will be storing the project if no database exists with the given url.
    /// Otherwise, simply connects to it
    /// Returns an in instance of a [GraphDatabase].
    ///
    /// ## Errors
    ///
    /// * [GraphDatabaseError::DatabaseError] if an unknown error was uncountered when trying to create/connect to it
    pub async fn connect_create_graph_database(
        db_url: &str,
        connection_options: Option<SqlxLogLevels>,
    ) -> Result<Self, GraphDatabaseError> {
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
    /// Returns an in instance of a [GraphDatabase].
    ///
    /// ## Errors
    ///
    /// Returns a:
    /// *   [GraphDatabaseError::DatabaseError] if something went wrong during the connection.
    pub async fn connect_graph_database(
        db_url: &str,
        connection_options: Option<SqlxLogLevels>,
    ) -> Result<Self, GraphDatabaseError> {
        // Install sqlite, postgre and mysql drivers
        sqlx::any::install_default_drivers();
        Ok(GraphDatabase::<T> {
            // obs: None,
            url: db_url.to_string(),
            pool: {
                let res = match connection_options {
                    Some(opt) => connect_with_options(db_url, opt).await,
                    None => sqlx::AnyPool::connect(db_url).await,
                };
                match res {
                    Ok(p) => p,
                    Err(e) => {
                        return Err(GraphDatabaseError::DatabaseError {
                            database_name: db_url.to_string(),
                            reason: e.to_string(),
                        })
                    }
                }
            },
            phantom_data: PhantomData,
        })
    }

    /// Closes the connection with the database (and drops this [`GraphDatabase`] instance)
    pub async fn close_connection(self) {
        self.pool.close().await
    }
}

/// Applies the given options to the pool *before* opening it.
async fn connect_with_options(
    url: &str,
    log_levels: SqlxLogLevels,
) -> Result<AnyPool, sqlx::Error> {
    let mut connect_opt = AnyConnectOptions::from_str(url)?;

    if let Some(level) = log_levels.log_statement_level {
        connect_opt = connect_opt.log_statements(level);
    }
    if let Some(level) = log_levels.log_slow_statement_level {
        connect_opt = connect_opt.log_slow_statements(level.0, level.1);
    }
    AnyPool::connect_with(connect_opt).await
}

// ______________________ DB WRITING ____________

impl<T: DbQuerySystem> GraphDatabase<T> {
    /// Add all canonical signatures to the table [`CANONICAL_TABLE_NAME`] of the dabase.
    /// Will not crash if a signature was already added previously.
    pub async fn add_to_dataset(
        &mut self,
        reader: impl BufRead,
        batch_size: usize,
    ) -> Result<(), GraphDatabaseError> {
        // Add dataset table if not already created
        let res = self.add_canonical_table().await;
        match &res {
            Ok(_) | Err(GraphDatabaseError::TableAlreadyCreatedError(_)) => {}
            _ => res?,
        }

        let push_to_db = async |vec: Vec<String>, batch_size| -> Result<(), GraphDatabaseError> {
            // Get insert query
            let query_str = T::get_insert_into_query(CANONICAL_TABLE_NAME, 2, batch_size);
            let safe_query = create_safe_query(query_str, vec)?;
            self.execute_query_no_return(safe_query).await?;
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
}

// ______________________ UTILS ______________________

impl<T: DbQuerySystem> GraphDatabase<T> {
    pub async fn get_all_table_names(&self) -> Result<Vec<String>, GraphDatabaseError> {
        let query_str = T::get_all_tables_query();

        let results: Vec<(String,)> = sqlx::query_as(&query_str)
            .fetch_all(&self.pool)
            .await
            .expect("no crash");

        Ok(results.iter().map(|(val,)| val.to_string()).collect())
    }

    async fn for_each_row(
        &self,
        table_name: String,
        apply_fn: &mut dyn FnMut(AnyRow) -> Result<(), GraphDatabaseError>,
    ) -> Result<(), GraphDatabaseError> {
        let query_str = T::get_all_from_table(table_name);

        let mut builder = QueryBuilder::<sqlx::Any>::new(query_str);

        // Execute query :
        let mut results = builder.build().fetch(&self.pool);

        while let Some(result_row) = results.next().await {
            match result_row {
                Ok(row) => apply_fn(row)?,
                Err(e) => {
                    return Err(GraphDatabaseError::DatabaseError {
                        database_name: self.url.to_string(),
                        reason: e.to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Adds the canonical table to the database.
    pub async fn add_canonical_table(&mut self) -> Result<(), GraphDatabaseError> {
        self.add_table(
            CANONICAL_TABLE_NAME,
            PK_NAME,
            ColumnType::String {
                max_size: Some(SIGNATURE_MAX_SIZE),
                default_value: None,
            },
            DATASET_VALUE_NAME,
            ColumnType::String {
                max_size: None,
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
    ) -> Result<(), GraphDatabaseError> {
        let query_str = T::get_create_table_query(
            table_name,
            pk_name,
            pk_column_type,
            value_name,
            value_column_type,
        );

        let mut builder = QueryBuilder::<sqlx::Any>::new(query_str);

        match builder.build().execute(&self.pool).await {
            Ok(v) => {
                debug!("Added signature table, result is : {:?}", v);
                Ok(())
            }
            Err(e) => Err(database_error::sqlx_error_to_db_error(e)),
        }
    }

    async fn execute_query_no_return(
        &self,
        mut query: sqlx::QueryBuilder<'static, sqlx::Any>,
    ) -> Result<(), GraphDatabaseError> {
        let res = query.build().execute(&self.pool).await;
        if let Err(e) = res {
            Err(database_error::sqlx_error_to_db_error(e))
        } else {
            Ok(())
        }
    }
}

// Will panic if there are more or less identifier/argument symbols than the given number.
fn create_safe_query(
    query_str: String,
    arguments: Vec<impl ToString>,
) -> Result<sqlx::QueryBuilder<'static, sqlx::Any>, GraphDatabaseError> {
    let mut builder = QueryBuilder::<sqlx::Any>::new("");
    let mut arguments = arguments.iter();

    for c in query_str.chars() {
        if c == '?' {
            match arguments.next() {
                Some(arg) => {
                    builder.push_bind::<String>(arg.to_string());
                }
                None => {
                    todo!("error !")
                }
            }
        } else {
            builder.push(c);
        }
    }
    Ok(builder)
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

impl<T: DbQuerySystem> GraphDatabase<T> {
    pub async fn print_all_tables(&self) -> Result<(), GraphDatabaseError> {
        // Get all tables :
        let tables = self.get_all_table_names().await?;
        if tables.is_empty() {
            println!("No tables in database");
            return Ok(());
        }
        for table in tables {
            let mut table_query: Option<QueryTable> = None;

            self.for_each_row(table, &mut |row| {
                // Fetch all header first
                if table_query.is_none() {
                    //
                    let mut headers = vec![];
                    for col in row.columns() {
                        headers.push(col.name().to_string());
                    }
                    table_query = Some(QueryTable::new(
                        headers,
                        crate::utils::table_handler::QueryTableOptions::Partial {
                            first_rows_count: 10,
                            last_rows_count: 10,
                        },
                    ));
                }
                // Add line to query
                table_query.as_mut().expect("is some").push_line({
                    let mut row_vec: Vec<String> = vec![];

                    for (i, col) in row.columns().iter().enumerate() {
                        match col.type_info().kind() {
                            sqlx::any::AnyTypeInfoKind::BigInt
                            | sqlx::any::AnyTypeInfoKind::Integer => {
                                let value: i64 = row.get(i);
                                row_vec.push(value.to_string());
                            }
                            _ => {
                                if let Ok(value) = row.try_get(i) {
                                    row_vec.push(value);
                                } else {
                                    row_vec.push("[?Cannot convert to string?]".to_string());
                                }
                            }
                        }
                    }

                    row_vec
                });
                Ok(())
            })
            .await?;
            if let Some(tab) = table_query {
                println!("{tab}");
            }
        }

        Ok(())
    }
}
