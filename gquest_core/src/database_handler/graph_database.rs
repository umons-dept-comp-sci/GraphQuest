use std::{fmt::Debug, str::FromStr, time::Duration};

use log::{debug, info};
use sqlx::{
    any::AnyConnectOptions, error::DatabaseError, migrate::MigrateDatabase, pool, query, Any,
    AnyPool, ConnectOptions, Database, Execute, Pool, QueryBuilder, Sqlite,
};

use crate::{
    database_handler::{database_error, DbQuerySystem, GraphDatabaseError, *},
    utils::subject::Observer,
};

/// Used to change the logging settings of a sqlx connection.
/// This is because by default, all debug options will be shown and will fill the logger really quickly. It is really recommended to set them to at least [`log::LevelFilter::Info`].
pub struct SqlxLogLevels {
    pub log_statement_level: Option<log::LevelFilter>,
    pub log_slow_statement_level: Option<(log::LevelFilter, Duration)>,
}

#[derive(Debug)]
/// Represents a graph database.
/// As long as it is not dropped, the connection to the related database will be stay up.
pub struct GraphDatabase<'a, T>
where
    T: DbQuerySystem,
{
    pool: Pool<sqlx::Any>,
    obs: Option<&'a dyn Observer>,
    query_db_system: T,
}

/// The GraphDatabase trait is used to facilitate the communication with databases for the user.
impl<'a, T: DbQuerySystem> GraphDatabase<'a, T> {
    /// Creates the database that will be storing the project.
    /// Returns an in instance of a [GraphDatabase].
    ///
    /// ## Errors
    ///
    /// * [GraphDatabaseError::DatabaseAlreadyCreated] if the database was already created
    /// * [GraphDatabaseError::DatabaseError] if an unknown error was uncountered when trying to create it
    pub async fn create_graph_database(
        db_url: &str,
        db_type: T,
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

        Self::connect_graph_database(db_url, db_type, connection_options).await
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
        db_type: T,
        connection_options: Option<SqlxLogLevels>,
    ) -> Result<Self, GraphDatabaseError> {
        // Install sqlite, postgre and mysql drivers
        sqlx::any::install_default_drivers();
        Ok(GraphDatabase {
            query_db_system: db_type,
            obs: None,
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

impl<'a, T: DbQuerySystem> GraphDatabase<'a, T> {
    /// Executes a query but does not look at the return value except for errors.
    /// Useful when you need to create a table, append data, ...
    async fn execute_query_no_return(
        &self,
        mut query: QueryBuilder<'_, Any>,
    ) -> Result<(), GraphDatabaseError> {
        let res = query.build().execute(&self.pool).await;
        if let Err(e) = res {
            return Err(database_error::sqlx_error_to_db_error(e));
        }
        Ok(())
    }
}

impl<'a, T: DbQuerySystem> GraphDatabase<'a, T> {
    pub async fn add_signature_table(&mut self) {
        let query_str = self
            .query_db_system
            .get_create_table_query(super::ColumnType::String {
                max_size: None,
                default_value: None,
            });

        // let mut builder = QueryBuilder::<sqlx::Any>::new(query_str);

        // for arg in [
        //     DATASET_TABLE_NAME,
        //     PK_NAME,
        //     SIGNATURE_MAX_SIZE,
        //     DATASET_VALUE_NAME,
        // ] {
        //     builder.push_bind(arg);
        // }

        // let query = builder.build();
        // println!("query: {:?}", query.sql());

        println!("query : {:?}", create_safe_query(query_str, [
            DATASET_TABLE_NAME,
            PK_NAME,
            SIGNATURE_MAX_SIZE,
            DATASET_VALUE_NAME,
        ].to_vec()))

        // query.execute(&self.pool).await.expect("whyyh ::(((");
    }
}

// Will panic if there are more or less bind symbols than the given number.
fn create_safe_query (
    query_str: String,
    arguments: Vec<&str>,
) -> Result<String, GraphDatabaseError> {
    let mut builder = QueryBuilder::<sqlx::Any>::new("");

    // let mut query = builder.build();
    let mut count = 0;

    for c in query_str.chars() {
        if c != '?' {
            builder.push(c);
        } else if count == arguments.len() {
            todo!("error");
        } else {
            builder
                .push_bind::<&str>(arguments.get(count).expect("shouldn't be out of bounds"));
            count += 1;
        }
    }
    Ok(builder.build().sql().to_string())
}
