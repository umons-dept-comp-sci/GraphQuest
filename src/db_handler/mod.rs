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

        debug_log(query.as_str());
        sqlx::query(&query).execute(&self.pool).await.unwrap()
    }

    /// Adds a column to the graph
    pub async fn add_column_to_table(&self, table_name: &str, column_name: &str, data_type: ColumnType)
    {
        let query = format!("ALTER TABLE {table_name} 
                                    ADD COLUMN {column_name} {};", data_type.to_sqlite());

        let result = sqlx::query(&query).execute(&self.pool).await;
        match result{
            Ok(_) => (),
            Err(_) => debug_log(format!("The table \"{table_name}\" already has the column \"{column_name}\", ignoring").as_str()),
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
/// * Create a database to the given path it if isn't already created
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







