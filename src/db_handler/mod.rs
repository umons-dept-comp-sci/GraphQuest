use std::fmt::Error;

use sqlx::{database, migrate::MigrateDatabase, pool, query, sqlite::{types, SqliteQueryResult}, FromRow, Pool, Row, Sqlite, SqlitePool};


const G6_FORMAT_MAX_SIZE: u8 = 250;
const DEBUG_MODE: bool = true;

/// Simple enum to manipulate the different sqlite data types
pub enum ColumnType {
    Text,
    Integer,
    Boolean 
}






/// Struct used to simplify the manipulation of database for the user.
pub struct GraphDatabase
{
    pool: Pool<Sqlite>
}

impl GraphDatabase {
    
    /// Creates a simple graph table that has only one column called **signature**.
    /// The goal being to store graphs by using their ***graph6***  format.
    pub async fn create_graph_table(&self, table_name: &str) -> SqliteQueryResult
    {
        let query = format!("CREATE TABLE IF NOT EXISTS {table_name} 
                                    (signature VARCHAR({G6_FORMAT_MAX_SIZE}) PRIMARY KEY NOT NULL);");

        sqlx::query(&query).execute(&self.pool).await.unwrap()
    }

    /// Tries to add a column to the graph \\
    /// # Errors
    /// Will panic if the specified table doesn't exist 
    pub async fn add_column_to_table(&self, table_name: &str, column_name: &str, data_type: ColumnType)
    {
        // check if table exists
        if let Err(msg) = self.check_if_table_exist(table_name).await
        {
            panic!("{msg}");
        }
        // Try to add the column
        let query = format!("ALTER TABLE {table_name} 
                                    ADD COLUMN {column_name} {};", data_type.to_sqlite());
            
        let error_msg = format!("The table \"{table_name}\" already has the column \"{column_name}\"");
        let r = self.match_query_result(&query, &error_msg).await; 

        match r {
            Ok(_) => (),
            Err(msg) => debug_log((msg + ", ignoring").as_str())
        }
    }

    
    pub async fn remove_column_from_table(&self, table_name: &str, column_name: &str)
    {
        // Check if table exists
        if let Err(msg) = self.check_if_table_exist(table_name).await
        {
            panic!("{msg}");
        }
                 
        let query = format!("ALTER TABLE {table_name} DROP COLUMN {column_name};");
        let result = sqlx::query(&query).execute(&self.pool).await;
        match result{
            Ok(_) => (),
            Err(_) => debug_log(format!("The table \"{table_name}\" doesn't have the column \"{column_name}\", ignoring").as_str()),
        }

    }

    /// Checks if the table is present in the database in the database
    /// * Returns `Ok(SqliteQueryResult)` if the table exists
    /// * Returns `Err(..)`if it doesn't
    async fn check_if_table_exist(&self, table_name: &str) -> Result<SqliteQueryResult, String>
    {
        self.match_query_result(format!("SELECT * FROM {table_name}").as_str(), 
                            "The table \"{table_name}\" does not exist for the given database")
                            .await
    }

    /// Tries the given query to the database
    /// * Returns `Ok(SqliteQueryResult)` if the query was a success
    /// * Returns `Err()` with the given error message if it wasn't
    async fn match_query_result(&self, query: &str, error_msg: &str) -> Result<SqliteQueryResult, String>
    {
        let res = sqlx::query(query)
                                                            .execute(&self.pool)
                                                            .await;
        match res{
            Ok(query_res) => 
                {return Ok(query_res);},
            Err(_) => 
                {return  Err(error_msg.to_string()) }
        }

    }

}


impl ColumnType {
    /// Returns the type as a string which can be use in a sqlite querry
    fn to_sqlite(&self) -> String
    {
        let res = match self {
            ColumnType::Text => "text",
            ColumnType::Integer | ColumnType::Boolean => "integer"
        };

        String::from(res)
    }
}


//#[derive(Clone, FromRow, Debug)]
//struct Graph {
//    signature: String
//}


fn debug_log(message: &str)
{
    if  DEBUG_MODE{
        println!("{}", message);
    }
}


/// Tries to create a database
/// * Creates a database to the given path it if isn't already created
/// * Doesn't do anything if it is already created.
pub async fn create_graph_database(db_path: &str)
{
    if !Sqlite::database_exists(db_path).await.unwrap_or(false) {
        debug_log(format!("Creating database {}", db_path).as_str());
        match Sqlite::create_database(db_path).await {
            Ok(_) => debug_log("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        debug_log("Database already exists, ignoring");   
    }
}


/// Tries to connect to an already existing database using the given *db_path*. Returns a [GraphDatabase] struct.
/// # Errors
/// Will `panic!(..)` if the given data path is not valid
pub async fn connect_graph_database(db_path : & str) -> GraphDatabase
{
    GraphDatabase{
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





