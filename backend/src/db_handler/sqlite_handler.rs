use std::fmt::Error;

use crate::{db_handler::graph_database::*, utils::subject::*};



use sqlx::{database, error::{DatabaseError, ErrorKind}, migrate::MigrateDatabase, pool, query, sqlite::{types, SqliteQueryResult}, Database, FromRow, Pool, Row, Sqlite, SqlitePool};


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
}





/// Struct used to simplify the manipulation of database for the user.
pub struct SqliteGraphDatabase<'a>
{
    pool: Pool<Sqlite>,
    obs: Vec<&'a dyn Observer>
}

impl<'a> Subject<'a> for SqliteGraphDatabase<'a>{
    fn set_graph_db_observer(&mut self, obs: &'a dyn Observer) {
        self.obs.push(obs);
    }

    fn remove_graph_db_observer(&mut self) {
        self.obs = vec![];
    }

    fn notify_observator(&self, progression: u64) {
        if self.obs.len() != 0 {
            self.obs[0].notify_data_pushed(progression);
        }
    }
    
    fn tick_observator(&self) {
        if self.obs.len() != 0 {
            self.obs[0].notify_tick();
        }
    }
}

impl<'a> GraphDatabase<'a> for SqliteGraphDatabase<'a> {
    // TODO Also create meta data table
    async fn create_graph_database(db_url: &str) -> Self {
        // Creates the database if it didn't already exists
        create_graph_database(db_url).await;
        // Create the object
        let db = SqliteGraphDatabase{
            pool:
            {
                // if managed to connected then return the value
                if let Ok(pool) = SqlitePool::connect(db_url).await{
                    pool
                }
                // else panic
                else {
                    panic!("The given database url is not valid")
                }
            },
            obs : Vec::new()
        };
        
        db
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
    
    async fn add_invariant_table<T: DBColumnTypes>(&self, invariant: &Invariant, column_type: T) {
        let query = format!("CREATE TABLE {} (
                                    {DATASET_PK_NAME} VARCHAR({SIGNATURE_MAX_SIZE}) PRIMARY KEY,
                                    value {}
                                ); ", invariant.get_table_name(), column_type.translate());
        sqlx::query(&query).execute(&self.pool).await.unwrap();
    }
    
    async fn add_value_to_dataset(&self, signatures: &Vec<String>) {
        // Then we add all signatures to the newly created table
        let mut query = format!("INSERT INTO {DATASET_TABLE_NAME} VALUES ");
        // Add all value to the query
        for sign in signatures
        {
            query.push_str(format!("(\"{sign}\"),").as_str());
        }
        query.pop();        // remove the extra ','
        query.push_str(";");
        sqlx::query(&query).execute(&self.pool).await.unwrap();
    }
    
    async fn fetch_dataset_signatures(&self, start_index: Option<usize>, end_index: Option<usize>) -> Vec<String> {
        todo!()
    }
    
    async fn connect_graph_database(db_url: &str) -> Self {
        SqliteGraphDatabase{
            pool:
            {
                // if managed to connected then return the value
                if let Ok(pool) = SqlitePool::connect(db_url).await{
                    pool
                }
                // else panic
                else {
                    panic!("The given database url is not valid")
                }
            },
            obs: Vec::new()
        }
        // TODO Check if the workspace is valid, if it wasn't tempered with
    }
    
    async fn add_signatures_to_dataset(&self, table_name: &str, signatures: &Vec<String>) {
        // Then we add all signatures to the newly created table
        let mut query = format!("INSERT INTO {table_name} VALUES ");
        let mut first_byte;
        // Add all value to the query
        for sign in signatures
        {
            first_byte = sign.as_bytes()[0];
            if first_byte >= 63 && first_byte < 126  
            {
                query.push_str(format!("(\"{sign}\", {}),", first_byte - 63).as_str());
            }
            else {
                todo!("Didn't implement what to do for large graph (n >= 62)");
            }
        }
        query.pop();        // remove the extra ','
        query.push_str(";");
        // Try to push data
        if let Err(e) = sqlx::query(&query).execute(&self.pool).await
        {
            react_to_database_error(&e);
        }
        // update meta data table
        self.update_meta_data(DATASET_TABLE_NAME, signatures.len()).await;
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

        /*
        INSERT OR REPLACE INTO users (id, username, email) VALUES (1, 'john doe', 'john@gmail.com');
         */
        let query = format!("INSERT OR REPLACE INTO {METADATA_TABLE_NAME} (table_name, stopped_at)
                                    VALUES (\"{changed_table_name}\", {new_value});");
        //println!("query : {query}");
        sqlx::query(&query).execute(&self.pool).await.unwrap();
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
        obs: Vec::new(), 
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
        ErrorKind::CheckViolation => todo!(),
        ErrorKind::Other => todo!(),
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