use sqlx::{error::ErrorKind, migrate::MigrateDatabase, Pool, Sqlite, SqlitePool};

use crate::{db_handler::{db_errors::GraphDatabaseError, graph_database::SIGNATURE_MAX_SIZE}, utils::subject::{Observer, Subject}};
use log::{debug, log};
use super::graph_database::{ColumnType, GraphDatabase};

pub struct SqliteGraphDatabase<'a>
{
    pool: Pool<Sqlite>,
    obs: Option<&'a dyn Observer>
}

impl<'a> Clone for SqliteGraphDatabase<'a> {
    fn clone(&self) -> Self {
        Self { pool: self.pool.clone(), obs: self.obs.clone() }
    }
}

impl<'a> Subject<'a> for SqliteGraphDatabase<'a>{
    fn set_graph_db_observer(&mut self, obs: &'a dyn Observer) {
        self.obs = Some(obs);
    }

    fn remove_graph_db_observer(&mut self) {
        self.obs = None;
    }

    fn update_observator(&self, progression: u64) {
        if let Some(o) = self.obs {
            o.notify_data_pushed(progression);
        }
    }
    
    fn tick_observator(&self) {
        if let Some(o) = self.obs {
            o.notify_tick();
        }
    }
}


impl<'a> GraphDatabase<'a> for SqliteGraphDatabase<'a> {
    async fn create_graph_database(db_url: &str) -> Result<Self, super::db_errors::GraphDatabaseError> {
        // Creates the database if it didn't already exists
        
        if !Sqlite::database_exists(db_url).await.unwrap_or(false) {
            debug!("Creating database {}", db_url);
            match Sqlite::create_database(db_url).await {
                Ok(_) => {debug!("Sucess creating the database {}", db_url);},
                Err(error) => 
                {
                    return Err(GraphDatabaseError::UnknownError { error_message: error.to_string() });
                }
            }
        } else {
                return Err(GraphDatabaseError::DatabaseAlreadyCreated { database_name: db_url.to_string() });
        }

        let res = Self::connect_graph_database(db_url).await;
        if let Err(e) = res {
            return Err(e);
        }
        
        // Return the created struct
        Ok(res.unwrap())
    }

    async fn connect_graph_database(db_url: &str) -> Result<Self, super::db_errors::GraphDatabaseError> {
        Ok(SqliteGraphDatabase{
            pool:
            {
                // if managed to connected then return the value
                if let Ok(pool) = SqlitePool::connect(db_url).await{
                    pool
                }
                // else panic
                else {
                    return Err(GraphDatabaseError::DatabaseNotFound { database_name: db_url.to_string() });
                }
            },
            obs: None, 
        })
    }

    async fn close_connection(self) {
        self.pool.close().await;
    }



    async fn update_meta_data(&self, changed_table_name: &str, added_values: usize) {
        todo!()
    }

    

    async fn fetch_data(&self, start_index: Option<usize>, end_index: Option<usize>, inv_names_to_join: &Vec<String>, inputs: &Vec<std::process::ChildStdin>) -> Result<(), super::db_errors::GraphDatabaseError> {
        todo!()
    }

    async fn execute_query(&self, query: &String, f: impl FnMut(Vec<String>, Vec<Vec<String>>)) {
        todo!()
    }

    async fn inv_already_added(&self, inv: &String) -> bool {
        todo!()
    }

    
    async fn get_size_of_table(&self, name: &str) -> Result<usize, super::db_errors::GraphDatabaseError> {
        todo!()
    }

    async fn join_tables(&self, new_table_name: &str, table_names: Vec<String>) -> Result<(), super::db_errors::GraphDatabaseError> {
        todo!()
    }

    fn get_all_tables_query(&self) -> String {
        String::from("SELECT name FROM sqlite_master WHERE type='table';")
    }
    
    
    async fn execute_query_no_return(&self, query: &String) -> Result<(),GraphDatabaseError> 
    {
        let res = sqlx::query(&query).execute(&self.pool).await;
        if let Err(e) = res{
            return Err(database_error_to_query_error(&e));
        }
        Ok(())
    }


    fn get_create_table_query(table_name: &str, pk_name: &str, value_name: &str, value_type: ColumnType ) -> String
    {   
        let value_column = match value_type {
            ColumnType::String { max_size, default_value } => {
                let mut tmp = String::from("VARCHAR");
                if let Some(m) = max_size {
                    tmp.push_str(format!("({})", m).as_str());
                }
                if let Some(d) = default_value {
                    tmp.push_str(format!(" DEFAULT {}", d).as_str());
                }
                tmp
            },
            ColumnType::Integer { default_value } => {
                let mut tmp = String::from("INTEGER");

                if let Some(d) = default_value {
                    tmp.push_str(format!(" DEFAULT {}", d).as_str());
                }
                tmp
            },
            ColumnType::Boolean { default_value } => {
                let mut tmp = String::from("INTEGER");
                // Convert true to 1 and false to 0
                if let Some(d) = default_value {
                    tmp.push_str(format!(" DEFAULT {}", {if d {
                        1
                    }else {
                        0
                    }}).as_str());
                }
                tmp
            },
        };


        format!("CREATE TABLE {table_name} ({pk_name} VARCHAR({SIGNATURE_MAX_SIZE}) PRIMARY KEY NOT NULL,
                                            {value_name} {value_column})")
    }


    fn get_insert_into_query(table_name: &str, signatures_values: &Vec<(String, String)>) -> String
    {
        let mut query = format!("INSERT OR REPLACE INTO {table_name} VALUES ");

        
        // Add all value to the query
        // The format is always: ("signature", value), ...
        for (sign, value) in signatures_values {
            query.push_str(format!("( \"{sign}\", {value}),").as_str());
        }
        
        query.pop();        // remove the extra ','
        query.push_str(";");
        query
    }

    
    fn get_delete_table_query(table_name: &str) -> String
    {
        format!("DROP TABLE {}", table_name)
    }


}



/// Tries to get the error code located inside the sqlx error
fn get_error_kind(e: &sqlx::Error) -> Result<ErrorKind, String>
{
    match e {
        sqlx::Error::Database(db_e) => {
            Ok(db_e.kind())
        },
        _ => Err(e.to_string()),
    }
}

fn database_error_to_query_error(e: &sqlx::Error) -> GraphDatabaseError
{
    let kind = get_error_kind(e).expect("Error not handled");
    match kind {
        ErrorKind::UniqueViolation => GraphDatabaseError::QueryError { error_message: format!("A signature was already inside the dataset") },
        ErrorKind::ForeignKeyViolation => GraphDatabaseError::QueryError { error_message: format!("A ForeignKeyViolation was encountered when trying to execute the query")},
        ErrorKind::NotNullViolation => GraphDatabaseError::QueryError { error_message: format!("A NotNullViolation was encountered when trying to execute the query")},
        ErrorKind::CheckViolation => GraphDatabaseError::QueryError { error_message: format!("A CheckViolation was encountered when trying to execute the query")},
        _ => GraphDatabaseError::QueryError { error_message: format!("An unknown error was encountered : {:?}", e)},
    }
}
