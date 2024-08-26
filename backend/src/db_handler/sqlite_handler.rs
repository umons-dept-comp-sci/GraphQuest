use std::io::Write;
use std::pin::Pin;
use std::process::ChildStdin;
use std::str::FromStr;
use std::time::Duration;

use crate::{db_handler::graph_database::*, utils::subject::*};
use crate::data_handler::invariant_handlers::*;

use futures::{Stream, StreamExt};
use sqlx::pool::PoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::ConnectOptions;
use sqlx::{error::ErrorKind, migrate::MigrateDatabase, sqlite::SqliteQueryResult, Pool, Sqlite, SqlitePool};

use super::db_errors::TableNotFoundError;


const DEBUG_MODE: bool = true;

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
    
    fn get_integer_column() -> Self {
        SqliteColumnType::Integer
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
    async fn create_graph_database(db_url: &str) -> Self {
        // Creates the database if it didn't already exists
        create_graph_database(db_url).await;
        // Create the object
        connect_graph_database(db_url).await
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
    
    
    
    async fn fetch_data(&self, start_index: Option<usize>, limit: Option<usize>, dependencies_to_join: &Vec<String>, mut inputs: Vec<ChildStdin>) -> Result<(), TableNotFoundError>
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
        let join_query_res: Result<String, TableNotFoundError> = {
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
        
        //println!("::->{}", join_query);
        
        // execute query
        let mut que_res: Pin<Box<dyn Stream<Item = Result<String, sqlx::Error>> + Send>> = sqlx::query_scalar(&join_query).fetch(&self.pool);
        
        // push result to the given stdout
        while let Some(res) = que_res.next().await
        {
            if let Ok(mut sign) = res 
            {
                
                sign.push('\n');
                // write in all inputs
                for input in &mut inputs {
                    input.write(sign.as_bytes()).unwrap();
                }
            }
        }   
        Ok(())
        
    }
    
    async fn connect_graph_database(db_url: &str) -> Self {
        SqliteGraphDatabase{
            pool:
            {
                // disables the log slow statements
                let c = SqliteConnectOptions::from_str(db_url).expect(format!("The given database url is not valid \"{db_url}\"").as_str())
                                        .log_slow_statements(log::LevelFilter::Off, Duration::from_secs(10));
                
                // if managed to connected then return the value
                if let Ok(pool) = SqlitePool::connect_with(c).await{
                    pool
                }
                // else panic
                else {
                    panic!("The given database url is not valid \"{db_url}\"")
                }
            },
            obs: None
        }
        // TODO Check if the workspace is valid, if it wasn't tempered with
    }
    
    async fn add_values_to_table(&self, table_name: &str, signatures_values: &Vec<(String, String)>) {
        // Then we add all signatures to the newly created table
        let mut query = format!("INSERT INTO {table_name} VALUES ");

        
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
                                    value {}); ", 
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
    
    async fn get_size_of_table(&self, name: &str) -> Result<usize, TableNotFoundError> {
        let query = format!("SELECT count({}) FROM {};", DATASET_PK_NAME, name);
        //println!("query: {:?}", &query);

        let res: Result<u64, sqlx::Error> = sqlx::query_scalar(&query).fetch_one(&self.pool).await;

        match res {
            Ok(count) => Ok(count as usize),
            Err(_) => Err(TableNotFoundError::new(name.to_string())),
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



fn debug_log(message: &str)
{
    if  DEBUG_MODE{
        println!("{}", message);
    }
}


async fn create_graph_database(db_path: &str)
{
    if !Sqlite::database_exists(db_path).await.unwrap_or(false) {
        debug_log(format!("Creating database {}", db_path).as_str());
        match Sqlite::create_database(db_path).await {
            Ok(_) => debug_log("Successfully created the database"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        panic!("The given database was already created");
    }
}


/// Tries to connect to an already existing database using the given *db_path*. Returns a [GraphDatabase] struct.
/// # Errors
/// Will `panic!(..)` if the given data path is not valid
pub async fn connect_graph_database<'a>(db_path : & str) -> SqliteGraphDatabase<'a>
{
    SqliteGraphDatabase{
        pool:
        {
            // if managed to connected then return the value
            if let Ok(pool) = SqlitePool::connect(db_path).await{
                pool
            }
            // else panic
            else {
                panic!("The given database path is not valid")
            }
        },
        obs: None, 
    }
}






// TODO Change all unwrap with match cases
// TODO Make better errors



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