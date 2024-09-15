use std::any::{Any, TypeId};
use std::fmt::{format, Debug};
use std::io::Write;
use std::pin::Pin;
use std::process::ChildStdin;
use std::str::FromStr;
use std::time::Duration;

use crate::{db_handler::graph_database::*, utils::subject::*};
use crate::data_handler::invariant_handlers::*;

use futures::{Stream, StreamExt};
use log::{debug, log};
use sqlx::pool::PoolOptions;
use sqlx::sqlite::{SqliteColumn, SqliteConnectOptions, SqlitePoolOptions, SqliteRow, SqliteTypeInfo};
use sqlx::{Column, ConnectOptions, Row, TypeInfo};
use sqlx::{error::ErrorKind, migrate::MigrateDatabase, sqlite::SqliteQueryResult, Pool, Sqlite, SqlitePool};

use super::db_errors::GraphDatabaseError::{self, *};



/// Simple enum to manipulate the different sqlite data types
pub enum SqliteColumnType {
    Varchar(usize),
    Text,
    Integer,
    Boolean 
}

impl DBColumnTypes for SqliteColumnType {
    
    fn translate(&self) -> String
    {
        let res = match self {
            SqliteColumnType::Text => "text",
            SqliteColumnType::Integer | SqliteColumnType::Boolean => "integer",
            SqliteColumnType::Varchar(value) => &format!("VARCHAR({})", &value.to_string()),
        };

        String::from(res)
    }
}





/// Struct used to simplify the manipulation of database for the user.
pub struct SqliteGraphDatabase<'a>
{
    pool: Pool<Sqlite>,
    obs: Option<&'a dyn Observer>
}

unsafe impl<'a> Send for SqliteGraphDatabase<'a> {}

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

    fn update_observator(&self, progression: u64, index: Option<usize>) {
        if let Some(o) = self.obs {
            o.notify_data_pushed(progression, index);
        }
    }
    
    fn tick_observator(&self, index: Option<usize>) {
        if let Some(o) = self.obs {
            o.notify_tick(index);
        }
    }
}

impl<'a> GraphDatabase<'a> for SqliteGraphDatabase<'a> {
    // TODO Also create meta data table
    async fn create_graph_database(db_url: &str) -> Result<Self, GraphDatabaseError> {
        // Creates the database if it didn't already exists
        if let Err(e) = create_graph_database(db_url).await {
            return Err(e);
        }
        let res = Self::connect_graph_database(db_url).await;
        if let Err(e) = res {
            return Err(e);
        }
        
        Ok(res.unwrap())
        // Create the object
    }

    async fn create_dataset_table(&self) {
        // Adds a database table 
        let query = format!("CREATE TABLE {DATASET_TABLE_NAME} 
                                    ({DATASET_PK_NAME} VARCHAR({SIGNATURE_MAX_SIZE}) PRIMARY KEY NOT NULL,
                                     {DATASET_VALUE_NAME} VARCHAR);");

        sqlx::query(&query).execute(&self.pool).await.unwrap();
    }
    
    async fn create_meta_data_table(&self) {
        // Adds a database table 
        let query = format!("CREATE TABLE {METADATA_TABLE_NAME} 
                                    (table_name VARCHAR({TABLE_NAME_MAX_SIZE}) PRIMARY KEY NOT NULL,
                                     stopped_at {} DEFAULT 0);", SqliteColumnType::Integer.translate());

        sqlx::query(&query).execute(&self.pool).await.unwrap();
    }
    
    
    
    async fn fetch_data(&self, start_index: Option<usize>, limit: Option<usize>, dependencies_to_join: &Vec<String>, mut inputs: &Vec<ChildStdin>) -> Result<(), GraphDatabaseError>
    {

        // Represents the query where we fetch the desired signatures
        let signatures_query = {
            let start = match start_index {
                Some(nb) => nb,
                None => 0,
            };
            
            let mut tmp = format!("SELECT {} FROM {}", DATASET_PK_NAME, DATASET_TABLE_NAME);
            if let Some(size) = limit 
            {
                tmp.push_str(format!(" LIMIT {}", size).as_str());
            }
            tmp.push_str(format!(" OFFSET {}", start).as_str());
            
            tmp
        };
        
        // Represents the query where we join the desired invariant tables
        let join_query_res: Result<String, GraphDatabaseError> = {
            let mut tmp = format!("SELECT * FROM ({})", signatures_query);
            let mut inv_name;
            for name in dependencies_to_join {
                inv_name = InvariantsExecutable::get_table_name_from_string(&name);
                if let Err(t) = self.get_size_of_table(&inv_name).await {
                    return Err(t);
                }
                tmp.push_str(format!(" INNER JOIN {} USING ({})", inv_name, DATASET_PK_NAME).as_str());
            }
            tmp.push(';');
            Ok(tmp)
        };
        if let Err(t) = join_query_res{
            return Err(t);
        }
        
        let join_query = join_query_res.unwrap();
        
        
        // execute query
        let que_res = self.execute_fetch_query(&join_query, None, None, StdoutOptions::None, true).await;
        if let Err(e) = que_res {
            return Err(e);
        }
        else {
            let lines = que_res.unwrap().unwrap();
            for line in lines  {
                let mut str = line.join(" ");
                str.push('\n');
                for mut input in inputs {
                    input.write(str.as_bytes()).unwrap();
                    input.flush().expect("Could not flush stdin of process");   
                }
            }

        }
        
        Ok(())
        
    }
    
    async fn connect_graph_database(db_url: &str) -> Result<Self, GraphDatabaseError> {
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
    
    async fn add_values_to_table(&self, table_name: &str, signatures_values: &Vec<(String, String)>) {
        // Then we add all signatures to the newly created table
        let mut query = format!("INSERT OR REPLACE INTO {table_name} VALUES ");

        
        // Add all value to the query
        // The format is always: ("signature", value), ...
        for (sign, value) in signatures_values {
            query.push_str(format!("( \"{sign}\", {value}),").as_str());
        }
        
        query.pop();        // remove the extra ','
        query.push_str(";");
        
        // Try to push data
        if let Err(e) = sqlx::query(&query).execute(&self.pool).await
        {
            react_to_database_error(&e);
        }
        // update meta data table
        self.update_meta_data(DATASET_TABLE_NAME, signatures_values.len()).await;
    }
    
    async fn close_connection(self) {
        self.pool.close().await;
        
    }
    
    async fn update_meta_data(&self, changed_table_name: &str, added_values: usize) {
        let new_value = 
        {
            let query = format!("SELECT COUNT(*) FROM {changed_table_name}");
            let current_table_size: Option<i64> = sqlx::query_scalar(&query).fetch_optional(&self.pool).await.unwrap();

            current_table_size.expect(format!("Couldn't read the size of the given table {}", changed_table_name).as_str())
            - added_values as i64
        };

        let query = format!("INSERT OR REPLACE INTO {METADATA_TABLE_NAME} (table_name, stopped_at)
                                    VALUES (\"{changed_table_name}\", {new_value});");
        //println!("query : {query}");
        sqlx::query(&query).execute(&self.pool).await.unwrap();
    } 

    //_________________________________INVARIANTS_______________________________________________________________________________
    
    async fn create_invariant_table(&self, inv: &String) {
        let query = format!("CREATE TABLE {} (
                                    {DATASET_PK_NAME} VARCHAR({SIGNATURE_MAX_SIZE}) PRIMARY KEY,
                                    {} {}); ",
                                    InvariantsExecutable::get_table_name_from_string(inv), 
                                    InvariantsExecutable::get_table_name_from_string(inv),
                                    SqliteColumnType::Integer.translate());   // FIXME CHANGE DEFAULT INTEGER
        sqlx::query(&query).execute(&self.pool).await.unwrap();
    }
    
    async fn inv_already_added(&self, inv: &String) -> bool {
        let query = format!("SELECT count(name) FROM sqlite_master WHERE type='table' AND name='{}';", InvariantsExecutable::get_table_name_from_string(inv));
        let res: Result<u8, sqlx::Error> = sqlx::query_scalar(&query).fetch_one(&self.pool).await;
        
        match res {
            Ok(count) => count == 1,
            Err(e) => {react_to_database_error(&e); false},
        }

    }
    
    async fn get_size_of_table(&self, name: &str) -> Result<usize, GraphDatabaseError> {
        let query = format!("SELECT count({}) FROM {};", DATASET_PK_NAME, name);
        //println!("query: {:?}", &query);

        let res: Result<u64, sqlx::Error> = sqlx::query_scalar(&query).fetch_one(&self.pool).await;

        match res {
            Ok(count) => Ok(count as usize),
            Err(_) => Err(GraphDatabaseError::TableNotFoundError{table_name : name.to_string()}),
        }
    }
    

    //_________________________________QUERIES_______________________________________________________________________________
    async fn execute_query(&self, query: &String, mut save_data: impl FnMut(Vec<String>, Vec<Vec<String>>)) 
    {
        
        let mut que_res = sqlx::query(&query).fetch(&self.pool);
        
        // push result to the given stdout
        let mut headers : Vec<String> = vec![];
        let mut lines : Vec<Vec<String>> = vec![];

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
                        _ => ()//println!("{}", col.type_info().name())
                    }
                    
                }
                lines.push(line);
            }
            else if let Err(e) = res{
                panic!("Error occured while trying to execute the given query: \"{query}\": {}",e);
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
        
    }
    
    async fn get_all_table_names_query(&self) -> String {
        String::from("SELECT name FROM sqlite_master WHERE type='table';")
    }



    async fn delete_table(&self, table_name: &str) -> Result<(), GraphDatabaseError>
    {
        // check if the table exists
        if let Err(e) = self.get_size_of_table(table_name).await {
            return Err(e);
        }
        
        let query = format!("DROP TABLE {}", table_name);
        let que_res = sqlx::query(&query).execute(&self.pool).await;
 
        match que_res {
            Ok(_) => Ok(()),
            Err(e) => {react_to_database_error(&e); Ok(())},
        }
    }
    
}



impl<'a> SqliteGraphDatabase<'a> {

    async fn _test_query_database(&self, query:String) -> Result<SqliteQueryResult, sqlx::Error>
    {
        sqlx::query(&query).execute(&self.pool).await
    }
    /// Returns the pool contained inside the struct
    fn _get_pool(&self) -> &Pool<Sqlite>
    {
        &self.pool
    }
}




async fn create_graph_database(db_path: &str) -> Result<(), GraphDatabaseError>
{
    if !Sqlite::database_exists(db_path).await.unwrap_or(false) {
    debug!("Creating database {}", db_path);
        match Sqlite::create_database(db_path).await {
            Ok(_) => {debug!("Sucess creating the database {}", db_path); Ok(())},
            Err(error) => 
            {
                Err(GraphDatabaseError::UnknownError { error_message: error.to_string() })
            }
        }
    } else {
        Err(GraphDatabaseError::DatabaseAlreadyCreated { database_name: db_path.to_string() })
    }
}






/// Tries to get the error code located inside the sqlx error
fn get_error_kind(e: &sqlx::Error) -> Result< ErrorKind, String>
{
    match e {
        sqlx::Error::Database(db_e) => {
            Ok(db_e.kind())
        },
        _ => Err(e.to_string()),
    }
}

fn react_to_database_error(e: &sqlx::Error) 
{
    let kind = get_error_kind(e).expect("Error not handled");
    match kind {
        ErrorKind::UniqueViolation => panic!("A signature was already inside the dataset"),
        ErrorKind::ForeignKeyViolation => todo!(),
        ErrorKind::NotNullViolation => todo!(),
        ErrorKind::CheckViolation => panic!(""),
        ErrorKind::Other => panic!("An unmapped database error happened, perhaps the table name is incorrect ?"),
        _ => todo!(),
    }
}




//_______________________________________________
// Unit testing

/*
#[cfg(test)]
mod tests {
    
    use super::*;   // Import all
    use std::process::Command;
    
    const DB_PATH: &str = "src/db_handler/test_db/sqlite_test_";
    
    fn get_test_path(postfix: &str) -> String
    {
        DB_PATH.to_string() + postfix
    }
    
    fn delete_test_db(path: &str)
    {
        // Delete the created database
        Command::new("rm")
            .arg(&path)
            .spawn()
            .expect("Failed to remove the database");
    }

    #[sqlx::test]
    async fn create_graph_table_test(_: sqlx::SqlitePool) -> sqlx::Result<()> 
    {
        let path = get_test_path("0.db");
        // Creates database
        SqliteGraphDatabase::create_graph_database(&path).await;
        
        // Checks if it exist
        if !Sqlite::database_exists(&path).await.unwrap_or(false) {
            panic!();
        }

        delete_test_db(&path);

        Ok(())
    }

    #[sqlx::test]
    async fn connect_graph_database(pool: sqlx::SqlitePool) -> sqlx::Result<()>
    {
        let path = get_test_path("1.db");
        // Creates database
        SqliteGraphDatabase::create_graph_database(&path).await;
        
        // Tries to connect
        SqliteGraphDatabase::connect_graph_database(&path).await;


        delete_test_db(&path);
        
        Ok(())
    }
}




//#[cfg(test)]
//mod tests {
//    use super::*;   // Import all
//
//    #[sqlx::test]
//    async fn create_graph_table_test(pool: sqlx::SqlitePool) -> sqlx::Result<()> {
//        let db = GraphDatabase::create_graph_database(pool);
//
//        db.create_graph_table("test_table").await;
//        let query_res = db.query_return_string("SELECT name FROM sqlite_master WHERE type='table' AND name='test_table';").await.unwrap();
//        assert_eq!(Some("test_table".to_string()), query_res);  // check if table was indeed created
//        Ok(())
//    }
//
//
//} 
*/