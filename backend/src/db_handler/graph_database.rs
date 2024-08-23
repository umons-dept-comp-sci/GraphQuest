
use std::process::ChildStdin;
use std::{io::BufRead, marker::PhantomData};

use crate::utils::subject::{Subject, Observer};

use super::super::data_handler::data_loaders::*;
use super::super::data_handler::invariant_handlers::*;
use super::db_errors::*;

/// The maximum capacity of the vector before pushing and flushing its content
pub const BUFFER_VECTOR_MAX_SIZE : usize = 2000;
/// The name of the first created table of the dataset containing the initial dataset
pub const DATASET_TABLE_NAME : &str = "InitDataset";
/// The column name of the primary key of the dataset
pub const DATASET_PK_NAME : &str = "signature";
/// The name of the second column of the dataset 
pub const DATASET_VALUE_NAME : &str = "nb_of_vertices";
/// The name of the the metadata table
pub const METADATA_TABLE_NAME : &str = "Metadata";
/// The name of the primary key of the metadata table
pub const METADATA_PK_NAME : &str = "table_name";
/// The name of the second column of the metadata table
pub const METADATA_VALUE_NAME : &str = "stopped_at";
/// The name of the column in an invariant table where the values are stored 
pub const INVARIANT_COLUMN_NAME : &str = "value";


/// The maximum size of a signature to store in the dataset
pub const SIGNATURE_MAX_SIZE : usize = 250;
/// The maximum size of a table name in the dataset
pub const TABLE_NAME_MAX_SIZE : usize = 250;
/// The speed at which the observator will be notified (if any present on db)
const ITERATION_BEFORE_NOTIFY : u8 = 100;



/// Encapsulation of a database connection that facilitates querries
pub struct Workspace<'a,  T: GraphDatabase<'a>> {
    /// The GraphDatabase used to modify the database state  
    db : T,
    _t : PhantomData<&'a T>
}


/// Traits used to offer a selection of different possible data types for a database.
pub trait DBColumnTypes
{
    /// Translates the given struct as a string which can be used in a query in order to 
    /// represent a type from a database
    fn translate(&self) -> String;

    /// Returns the integer enum
    fn get_integer_column() -> Self;
} 

// TODO perhaps it would be usefull to specify the return value as a Result<..> ?

/// The GraphDatabase trait is used to facilitate the communication with databases for the user.
pub trait GraphDatabase<'a> : Subject<'a> + Clone + Send 
 {
    
    /// Creates the database that will be storing the project.
    /// 
    /// Returns a struct implementing the [GraphDatabase] trait.
    async fn create_graph_database(db_url: &str) -> Self;


    /// Connects to the given database
    /// 
    /// ## Exceptions
    /// Must panic when:
    /// *   The given database url is not valid
    /// *   The database is not a valid workspace, meaning it was mostlikely tempered with (#TODO)
    async fn connect_graph_database(db_url: &str) -> Self;

    /// Adds a dataset table to the database that will be used to store all initial signatures.
    /// The database table must only have one column (with it being the primary key).
    /// And must be named using the given parameters.
    /// ## Exemple of table
    /// ```text
    /// InitDataset -> | signature | nb_of_vertices |
    ///                +-----------+----------------+
    ///                | I?ABCd[v? |       7        |
    ///                | I?ABCd[n? |       7        |
    ///                | I?ABCd[^? |       7        |
    ///                |          ...               |
    /// ```
    async fn create_dataset_table(&self);


    async fn add_values_to_dataset(&self, signatures: &Vec<String>);

    /// Adds a metadata table to the database that will be used to store all initial signatures.
    /// 
    /// 
    /// And must be named using the given parameters.
    /// ## Exemple of table
    /// ```text
    /// Metadata -> | table_name  | stopped_at |
    ///             |-------------|------------|
    ///             | InitDataset | 1500       |
    ///             | Euler       | 753        |
    ///             |            ...           |
    /// ```
    async fn create_meta_data_table(&self);
    
    async fn update_meta_data(&self, changed_table_name: &str, added_values: usize);

    /// Adds value to the dataset table.
    /// 
    /// If the number of values to push is too big, consider using [GraphDatabase::add_signatures_to_dataset_buffer()] instead, which is also using this method.
    /// ## Exceptions
    /// Must panic when:
    /// * The given table name doesn't not exists, because [GraphDatabase::create_dataset_table()] was not called before
    /// * The values to add are not valid.
    /// * The values break the primary key rule (i.e. a signature is already inside the dataset)
    async fn add_values_to_table(&self, table_name: &str, signatures_values: &Vec<(String, String)>);



    /// Fetches signatures from the dataset table and writes them into the given input
    /// ## Args
    /// * `start_index` : The index of the table to start executing the data at
    ///     * If the given value is `none`, the fetching will start a 0 
    /// * `limit` : The limit on the number of value to fetch
    ///     * If the given value is `none`, the fetching will be stop at the end of the table
    /// * `inv_names_to_join` : The names of all invariants columns to join when fetching the data
    /// ## Exceptions
    /// Must panic when:
    /// * The given indexes are not valid
    async fn fetch_data(&self, start_index: Option<usize>, end_index: Option<usize>, inv_names_to_join: Vec<String>, input: &ChildStdin) -> Result<(), TableNotFoundError>;
    
    /// Reads line by line the given buffer and pushes it's content in the given datase.
    /// 
    /// In order to save memory, the method will use a vector to store the data read
    /// and after reaching a certain max capacity (being [BUFFER_VECTOR_MAX_SIZE]),
    /// will dump its content to the database using [GraphDatabase::fetch_dataset_signatures].
    async fn add_signatures_to_dataset_buffer(&self, reader: impl BufRead)
    {
        let mut signature_value_buffer : Vec<(String, String)> = Vec::new();
        
        let mut notif_countdown = 1;
        let mut byte_buffer: u64 = 0;
        for line in reader.lines() {
            
            if let Ok(sign) = line {
                
                byte_buffer += (sign.len() + 1) as u64; // Count bytes read, (+ 1 because we also read the '\n' char)

                // Also push the order of this graph as a value
                signature_value_buffer.push((sign.clone(),
                    {
                        let first_byte = sign.as_bytes()[0];
                        if first_byte >= 63 && first_byte < 126{
                            (first_byte - 63).to_string()
                        }
                        else{
                            todo!("Didn't implement what to do for large graph (n >= 62)");
                        }
                    }
                ));
            }else {
                panic!("Could not read next buffer line");
            }
            // if we stored enough, we can push what we collected towards the given database
            if signature_value_buffer.len() == BUFFER_VECTOR_MAX_SIZE {
                self.add_values_to_table(DATASET_TABLE_NAME, &signature_value_buffer).await; // add already stored signatures to the database
                signature_value_buffer.clear();   // free the *buffer*
            }
            notif_countdown -= 1;   // Update countdown
            // If it is time to notify the observor
            if notif_countdown == 0 {
                self.update_observator(byte_buffer);   
                notif_countdown = ITERATION_BEFORE_NOTIFY;  // Reset progression
                byte_buffer = 0;
            }
        }
        if signature_value_buffer.len() != 0
        {                
            self.update_observator(byte_buffer);   // Last notifications
            self.add_values_to_table(DATASET_TABLE_NAME, &signature_value_buffer).await;  // add remaining values to the database
        }
    }

    /// Push received data to an invariant table
    async fn push_data_from_buffer(&self, inv: &InvariantsExecutable, reader: impl BufRead)
    {
        // FIXME
        let mut signature_value_buffer : Vec<(String, String)> = Vec::new();
        
        let mut notif_countdown = 1;
        let mut byte_buffer: u64 = 0;
        for line in reader.lines() {
            
            if let Ok(sign_value) = line {
                
                byte_buffer += (sign_value.len() + 1) as u64; // Count bytes read, (+ 1 because we also read the '\n' char)
                let (sign, value) = {
                    let v: Vec<&str> = sign_value.split_ascii_whitespace().collect();
                    if v.len() > 2 {
                        panic!("Read {} values on a line, expected 2: (signature value)", v.len());
                    }
                    if v.len() == 0
                    {
                        break;
                    }
                    (v[0].to_string(), v[1].to_string())
                };
                
                signature_value_buffer.push((sign,value));
            }else {
                panic!("Could not read next buffer line");
            }
            // if we stored enough, we can push what we collected towards the given database
            if signature_value_buffer.len() == BUFFER_VECTOR_MAX_SIZE {
                //self.add_values_to_table(&inv.get_table_name(), &signature_value_buffer).await; // add already stored signatures to the database
                signature_value_buffer.clear();   // free the *buffer*
            }
            notif_countdown -= 1;   // Update countdown
            // If it is time to notify the observor
            if notif_countdown == 0 {
                self.update_observator(byte_buffer);   
                notif_countdown = ITERATION_BEFORE_NOTIFY;  // Reset progression
                byte_buffer = 0;
            }
        }
        if signature_value_buffer.len() != 0
        {                
            self.update_observator(byte_buffer);   // Last notifications
            //self.add_values_to_table(&inv.get_table_name(), &signature_value_buffer).await;  // add remaining values to the database
        }
    }

    /// Closes the connection with the database
    async fn close_connection(self);

    //_________________________________INVARIANTS_______________________________________________________________________________

    /// Creates the given invariant table and adds it to the meta data table,
    /// if it wasn't already added
    async fn init_invariant(&self, inv: &InvariantsExecutable)
    {
        if !self.inv_already_added(inv).await
        {
            // Create table
            // TODO, by default we use an int but we do need to check the return type of this invariant
            self.create_invariant_table(inv).await;
            // Add Metadata line
            // FIXME
            //self.update_meta_data(&inv.get_table_name(), 0).await;
        }
    }

    /// Checks if the invariant was already added to the database
    async fn inv_already_added(&self, inv: &InvariantsExecutable) -> bool;
    
    /// Adds an table to the database to later store the value of an invariant for each graph of the database.
    /// 
    /// An invariant table must have two columns named [DATASET_PK_NAME] (which is the primary key) and *value*.
    /// 
    /// To name the name use [Invariant::get_table_name], this is done to make sure the given invariant name is valid.
    /// 
    /// ## Exemple of table
    /// ```text
    /// Chromatic -> | signature | value |
    /// Number       +-----------+-------+
    ///              | I?ABCd[v? | ##### |
    ///              | I?ABCd[n? | ##### |
    ///              | I?ABCd[^? | ##### |
    ///              |          ...      |
    /// ```
    /// ## Exceptions
    /// Must panic when:
    /// * The given table name is already used
    async fn create_invariant_table(&self, invariant: &InvariantsExecutable);


    /// Simply returns the length of the table with the given name
    /// ## Exceptions
    /// Must return a TableNotFoundError if the given table is not found
    async fn get_size_of_table(&self, name: &str) -> Result<usize, TableNotFoundError>;
    

    /// Gets the minimum size between all given tables and the dataset table 
    /// ## Exceptions
    /// Will panic if one of the given table doesn't not exists
    async fn get_min_dependency_size(&self, tables: &Vec<String>) -> usize
    {
        // The starting min size is obviously the quantity of data stored inside the dataset
        let mut min_size = self.get_size_of_table(DATASET_TABLE_NAME).await.unwrap();
        
        // Get minimum
        let mut table_size;
        for table in tables {
            println!("going for : {:?}", table);
            table_size = self.get_size_of_table(table).await.expect("One the given table does not exists");
            if table_size < min_size {
                min_size = table_size;
            }
        }

        min_size
    }
}




impl<'a, T: GraphDatabase<'a>> Workspace<'a, T> {
    /// Initialises a workplace
    /// 
    /// Will create everything needed by the program by using :
    /// * [GraphDatabase::create_graph_database] : to create the database
    /// * [GraphDatabase::create_dataset_table] : to create the dataset table
    /// * [GraphDatabase::create_meta_data_table] : to create the meta data table
    pub async fn init_workspace(db_url: &str) -> Self
    {

        
        // Init database
        let db = T::create_graph_database(db_url).await;
        
        // Init dataset table
        db.create_dataset_table().await;

        // Init meta data table
        db.create_meta_data_table().await;

        // Return the db connection encapsulated
        Workspace {
            db,
            _t : Default::default()
        }
    }


    /// Connects to the workspace using the given url
    pub async fn connect_workspace(db_url: &str) -> Self
    {
        // Try to connect to the database
        let db = T::connect_graph_database(db_url).await;
        // Return the db connection encapsulated
        Workspace {
            db,
            _t : Default::default()
        }
    }


    pub async fn add_dataset(&mut self, method: Method, temp_obs: &'a dyn Observer )
    {
        self.db.set_graph_db_observer(temp_obs);
        method.read_signatures(&self.db).await;

        self.db.remove_graph_db_observer();
    }





    

    /// Properly closes the worspace
    pub async fn close_workspace(self)
    {
        self.db.close_connection().await;
    }

    pub async fn test(&self, inv: &InvariantsExecutable)
    {
        inv.exec_inv(&self.db).await;
    }
    

    pub async fn compute_invariants(&self, inv_manager : InvariantsExecManager, process_available: usize)
    {
        inv_manager.handle_process_executions(process_available).await;
    }
}