use futures::StreamExt;
use sqlx::{error::ErrorKind, migrate::MigrateDatabase, Column, Pool, Row, Sqlite, SqlitePool, TypeInfo};

use crate::{data_handler::invariant_handlers::InvariantsExecutable, db_handler::{db_errors::GraphDatabaseError, graph_database::SIGNATURE_MAX_SIZE}, utils::subject::{Observer, Subject}};
use log::debug;
use super::graph_database::{ColumnType, GraphDatabase, BUFFER_VECTOR_MAX_SIZE};




pub struct SqliteGraphDatabase<'a>
{
    pool: Pool<Sqlite>,
    obs: Option<&'a dyn Observer>
}

impl<'a> Clone for SqliteGraphDatabase<'a> 
{
    fn clone(&self) -> Self 
    {
        Self { pool: self.pool.clone(), obs: self.obs.clone() }
    }
}

impl<'a> Subject<'a> for SqliteGraphDatabase<'a>
{
    fn set_graph_db_observer(&mut self, obs: &'a dyn Observer) 
    {
        self.obs = Some(obs);
    }

    fn remove_graph_db_observer(&mut self) 
    {
        self.obs = None;
    }

    fn update_observator(&self, progression: u64) 
    {
        if let Some(o) = self.obs {
            o.notify_data_pushed(progression);
        }
    }
    
    fn tick_observator(&self) 
    {
        if let Some(o) = self.obs {
            o.notify_tick();
        }
    }
}


impl<'a> GraphDatabase<'a> for SqliteGraphDatabase<'a> 
{
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

    async fn execute_query_no_return(&self, query: &String) -> Result<(),GraphDatabaseError> 
    {
        let res = sqlx::query(&query).execute(&self.pool).await;
        if let Err(e) = res{
            return Err(database_error_to_query_error(query.to_string(), &e));
        }
        Ok(())
    }


    async fn execute_query(&self, query: &String, mut save_data: impl FnMut(Vec<String>, Vec<Vec<String>>)) -> Result<(), GraphDatabaseError> 
    {
        let mut que_res = sqlx::query(&query).fetch(&self.pool);
        
        // push result to the given stdout
        let mut headers : Vec<String> = vec![];
        let mut lines : Vec<Vec<String>> = vec![];

        // For each result in the returned stream
        while let Some(res) = que_res.next().await
        {
            if let Ok(sign) = res 
            {
                let mut line: Vec<String> = vec![];
                // get vector with the column names
                if headers.len() == 0 {
                    
                    for col in sign.columns(){
                        headers.push(col.name().to_string());
                    }
                }
                for (i, col) in sign.columns().iter().enumerate() {
                    
                    match col.type_info().name() {
                        "INTEGER" | "NULL" => {
                            let value: i64 = sign.get(i);
                            //println!("res : {:?}", value);
                            line.push(value.to_string());
                        },
                        "TEXT" => {
                            let value: String = sign.get(col.name());
                            //println!("res : {:?}", value);
                            line.push(value);
                        },
                        _ => {return Err(GraphDatabaseError::UnknownError { error_message: format!("Received column with an unknown type: {:?}", col) });}
                    }
                    
                }
                lines.push(line);
            }
            else if let Err(e) = res{
                return Err(GraphDatabaseError::QueryError { query: query.to_string(), error_message: format!("{:?}", e) });
            }
            if lines.len() == BUFFER_VECTOR_MAX_SIZE {
                save_data(headers.clone(), lines);
                lines = vec![];
            }
        }   
        // Push last lines
        if lines.len() != 0 {
            save_data(headers.clone(), lines);
        }
        Ok(())
    }
    
    
    async fn inv_already_added(&self, inv: &String) -> Result<bool, GraphDatabaseError> 
    {
        let query = format!("SELECT count(name) FROM sqlite_master WHERE type='table' AND name='{}';", InvariantsExecutable::get_table_name_from_string(inv));
        let res: Result<u8, sqlx::Error> = sqlx::query_scalar(&query).fetch_one(&self.pool).await;
        
        match res {
            Ok(count) => Ok(count == 1),
            Err(e) => {return Err(database_error_to_query_error(query, &e));},
        }
    }

    
    async fn get_size_of_table(&self, name: &str) -> Result<usize, GraphDatabaseError> 
    {
        let query = format!("SELECT count(*) FROM {};", name);
        
        let res: Result<u64, sqlx::Error> = sqlx::query_scalar(&query).fetch_one(&self.pool).await;

        match res {
            Ok(count) => Ok(count as usize),
            Err(_) => Err(GraphDatabaseError::TableNotFoundError{table_name : name.to_string()}),
        }
    }
    


    
    async fn join_save_tables(&self, new_table_name: &str, table_names: Vec<String>, common_column_name: &str) -> Result<(), GraphDatabaseError>
    {
        let mut query = format!("CREATE TABLE IF NOT EXISTS {} AS ", new_table_name);
        
        query.push_str(Self::get_join_table_query(table_names, common_column_name).as_str());

        self.execute_query_no_return(&query).await
    }
    


    //__________________GETTERS_________________________________



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



    fn get_all_tables_query(&self) -> String {
        String::from("SELECT name FROM sqlite_master WHERE type='table';")
    }
    
    /// Returns the query that can be used to fetch a table made of all the given tables joined.
    /// The vector "*table_names*" must have at least one element
    fn get_join_table_query(table_names: Vec<String>, common_column_name: &str) -> String
    {

        let mut tmp = format!("SELECT * FROM ({})", table_names[0]);
        for i in 1..table_names.len() {
            tmp.push_str(format!(" INNER JOIN {} USING ({})", table_names[i], common_column_name).as_str());
        }
        
        tmp
    }

    fn get_all_rows_from_table_column(table_name: &str, column_name: &str) -> String
    {
        format!("SELECT {} FROM {}", column_name, table_name)
    }


    fn get_select_batch_from(from_table: String, start_index: Option<usize>, limit: Option<usize>) -> String
    {
        let start = match start_index {
            Some(nb) => nb,
            None => 0,
        };
        
        let mut tmp = format!("SELECT * FROM ({})", from_table);
        if let Some(size) = limit 
        {
            tmp.push_str(format!(" LIMIT {}", size).as_str());
        }
        tmp.push_str(format!(" OFFSET {}", start).as_str());
        
        tmp
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

fn database_error_to_query_error(query: String, e: &sqlx::Error) -> GraphDatabaseError
{
    let kind = get_error_kind(e).expect("Error not handled");
    match kind {
        ErrorKind::UniqueViolation => GraphDatabaseError::QueryError { query, error_message: format!("A signature was already inside the dataset") },
        ErrorKind::ForeignKeyViolation => GraphDatabaseError::QueryError { query,  error_message: format!("A ForeignKeyViolation was encountered when trying to execute the query")},
        ErrorKind::NotNullViolation => GraphDatabaseError::QueryError { query,  error_message: format!("A NotNullViolation was encountered when trying to execute the query")},
        ErrorKind::CheckViolation => GraphDatabaseError::QueryError { query,  error_message: format!("A CheckViolation was encountered when trying to execute the query")},
        _ => GraphDatabaseError::QueryError { query,  error_message: format!("An unknown error was encountered : {:?}", e)},
    }
}
