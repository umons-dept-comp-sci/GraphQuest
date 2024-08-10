use std::fmt::{Debug, Error};


use crate::db_handler::lib::*;



use sqlx::{database, migrate::MigrateDatabase, pool, query, sqlite::{types, SqliteQueryResult}, FromRow, Pool, Row, Sqlite, SqlitePool};


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
pub struct SqliteGraphDatabase
{
    pool: Pool<Sqlite>
}

impl GraphDatabase for SqliteGraphDatabase {
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
            } 
        };
        
        db
    }

    async fn create_dataset_table(&self, table_name: &str, pk_name: &str) {
        // Adds a database table 
        let query = format!("CREATE TABLE {table_name} 
                                    ({pk_name} VARCHAR({SIGNATURE_MAX_SIZE}) PRIMARY KEY NOT NULL);");

        sqlx::query(&query).execute(&self.pool).await.unwrap();
    }
    
    async fn create_meta_data_table(&self, table_name: &str) {
        // Adds a database table 
        let query = format!("CREATE TABLE {table_name} 
                                    (table_name VARCHAR({TABLE_NAME_MAX_SIZE}) PRIMARY KEY NOT NULL,
                                     stopped_at {} DEFAULT 0);", SqliteColumnType::Integer.translate());

        sqlx::query(&query).execute(&self.pool).await.unwrap();
    }
    
    async fn add_invariant_table<T: DBColumnTypes>(&self, invariant: &Invariant, column_type: T) {
        let query = format!("CREATE TABLE {} (
                                    {DATASET_PK_NAME} VARCHAR({SIGNATURE_MAX_SIZE}) PRIMARY KEY,
                                    value {}
                                ); ", invariant.get_table_name(), column_type.translate());
        println!("WHAT");
        sqlx::query(&query).execute(&self.pool).await.unwrap();
    }
    
    async fn add_value_to_dataset(&self, table_name: &str, signatures: &Vec<String>) {
        // Then we add all signatures to the newly created table
        let mut query = format!("INSERT INTO {table_name} VALUES ");
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
    
    async fn add_values_to_inv_table<T>(&self, invariant: &Invariant, values: &Vec<T>) {
        todo!()
    }
    

    
}



fn debug_log(message: &str)
{
    if  DEBUG_MODE{
        println!("{}", message);
    }
}


/// Tries to create a database
/// * Creates a database to the given path it if isn't already created
/// * Doesn't do anything if it is already created.
async fn create_graph_database(db_path: &str)
{
    if !Sqlite::database_exists(db_path).await.unwrap_or(false) {
        debug_log(format!("Creating database {}", db_path).as_str());
        match Sqlite::create_database(db_path).await {
            Ok(_) => debug_log("Successfully created the database"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        debug_log("Database already exists, ignoring");   
    }
}


/// Tries to connect to an already existing database using the given *db_path*. Returns a [GraphDatabase] struct.
/// # Errors
/// Will `panic!(..)` if the given data path is not valid
pub async fn connect_graph_database(db_path : & str) -> SqliteGraphDatabase
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
        } 
    }
}


// TODO Change all unwrap with match cases
// TODO Make better errors






//_______________________________________________
// Unit testing

