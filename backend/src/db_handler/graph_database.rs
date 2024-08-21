
use std::{io::BufRead, marker::PhantomData};

use crate::utils::subject::{Subject, Observer};

use super::super::data_handler::data_loaders::*;
use super::super::data_handler::invariant_handlers::*;


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


/// Traits used to offer a selection of different possible data types for a database
pub trait DBColumnTypes
{
    /// Translates the given struct as a string which can be used in a query in order to 
    /// represent a type from a database
    fn translate(&self) -> String;
} 


// TODO perhaps it would be usefull to specify the return value as a Result<..> ?

/// The GraphDatabase trait is used to facilitate the communication with databases for the user.
pub trait GraphDatabase<'a> : Subject<'a>
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


    async fn add_value_to_dataset(&self, signatures: &Vec<String>);

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
    async fn add_signatures_to_dataset(&self, table_name: &str, values: &Vec<String>);

    /// Fetches signatures from the dataset table and executes the given function
    /// ## Args
    /// * `start_index` : The index of the table to start executing the data at
    ///     * If the given value is `none`, the fetching will start a 0 
    /// * `f` : The function to call for each signature in the dataset
    /// ## Exceptions
    /// Must panic when:
    /// * The given index is not valid
    async fn fetch_dataset_signatures(&self, start_index: Option<usize>, f: &dyn Fn(String));
    
    /// Reads line by line the given buffer and pushes it's content in the given datase.
    /// 
    /// In order to save memory, the method will use a vector to store the data read
    /// and after reaching a certain max capacity (being [BUFFER_VECTOR_MAX_SIZE]),
    /// will dump its content to the database using [GraphDatabase::fetch_dataset_signatures].
    async fn add_signatures_to_dataset_buffer(&self, reader: impl BufRead)
    {
        let mut signature_buffer : Vec<String> = Vec::new();
        let mut notif_countdown = 1;
        let mut byte_buffer: u64 = 0;
        for line in reader.lines() {
            
            if let Ok(sign) = line {
                
                byte_buffer += (sign.len() + 1) as u64; // Count bytes read, (+ 1 because we also read the '\n' char)
                signature_buffer.push(sign);
            }else {
                panic!("Could not read next buffer line");
            }
            // if we stored enough, we can push what we collected towards the given database
            if signature_buffer.len() == BUFFER_VECTOR_MAX_SIZE {
                self.add_signatures_to_dataset(DATASET_TABLE_NAME, &signature_buffer).await; // add already stored signatures to the database
                signature_buffer.clear();   // free the *buffer*
            }
            notif_countdown -= 1;   // Update countdown
            // If it is time to notify the observor
            if notif_countdown == 0 {
                self.update_observator(byte_buffer);   
                notif_countdown = ITERATION_BEFORE_NOTIFY;  // Reset progression
                byte_buffer = 0;
            }
        }
        if signature_buffer.len() != 0
        {                
            self.update_observator(byte_buffer);   // Last notifications
            self.add_signatures_to_dataset(DATASET_TABLE_NAME, &signature_buffer).await;  // add remaining values to the database
        }
    }

    /// Closes the connection with the database
    async fn close_connection(self);

    //_________________________________INVARIANTS_______________________________________________________________________________

    /// Creates the given invariant table and adds it to the meta data table 
    async fn init_invariant(&self, inv: &Invariant)
    {
        // Create table
        //self.create_invariant_table(inv, column_type).await;
        // Add Metadata line
        self.update_meta_data(&inv.get_table_name(), 0).await;
    }

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
    async fn create_invariant_table<T: DBColumnTypes>(&self, invariant: &Invariant, column_type: T);
    
    /// Launches a thread that computes the given invariant
    /// 
    /// * Must create the invariant table if it does not already exists [GraphDatabase::inver]
    /// * If the invariant table was partially completed, it must continue from where it previously stopped 
    async fn launch_thread_invariant(&self, inv: &Invariant);
    
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

    pub async fn test(&self)
    {
        self.db.fetch_dataset_signatures(Some(0), &|x|  println!("Read: {}", x)).await;
    }

    

    pub async fn compute_invariants(&self, inv_order : InvariantsOrderHandler)
    {
        let x = inv_order.get_topological_order();
        //x.handle_execution(6);
    }
}