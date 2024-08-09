use core::fmt;
use std::{io::BufRead, path::Display};


/// The maximum capacity of the vector before pushing and flushing its content
pub const BUFFER_VECTOR_MAX_SIZE : usize = 2000;
/// The name of the first created table of the dataset containing the initial dataset
pub const DATASET_TABLE_NAME : &str = "InitDataset";
/// The maximum size of a signature to store in the dataset
pub const SIGNATURE_MAX_SIZE : usize = 250;
/// The prefix of all the invariant tables 
pub const INVARIANT_PREFIX : &str = "inv_";



// FIXME ATTENTION USER INPUT ET TABLE NAMES,


/// A simple struct used to force a specific naming convention for tables storing invariants
pub struct Invariant {
    name : String,
}


impl Invariant {
    /// Creates an invariant using the given name
    pub fn create_invariant(name: &str) -> Self
    {
        Self {
            name : name.to_string()
        }
    }
    // Simply formats the invariant name to be easier to work with
    pub fn get_table_name(&self) -> String
    {
        INVARIANT_PREFIX.to_string() + &self.name    // Append the prefix to the invariant 
    }
}

//impl fmt::Display for Invariant {
//    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//        write!(f,"{}", self.get_table_name())
//    }
//}


/// Traits used to offer a selection of different possible data types for a database
pub trait DBColumnTypes
{
    /// Translates the given struct as a string which can be used in a query in order to 
    /// represent a type from a database
    fn translate(&self) -> String;
} 


// TODO perhaps it would be usefull to specify the return value as a Result<..> ?

/// The GraphDatabase trait is used to facilitate the communication with databases for the user.
pub trait GraphDatabase {
    
    /// Creates the database that will be storing the project.
    /// Adds the dataset table that will be used to store all initial signatures. 
    /// Returns a struct implementing the [GraphDatabase] trait.
    /// 
    /// The database table must only have one column named *signature* (with it being the primary key).
    /// And must be called [DATASET_TABLE_NAME]
    /// ```text
    /// [DATASET_TABLE_NAME]->   | signature |
    ///                          +-----------+
    ///                          | I?ABCd[v? |
    ///                          | I?ABCd[n? |
    ///                          | I?ABCd[^? |
    ///                          |    ...    |
    /// ```
    async fn create_graph_database(db_url: &str) -> Self;
    
    /// Adds an table to the database to later store the value of an invariant for each graph of the database.
    /// 
    /// An invariant table must have two columns named *signature* (which is the primary key) and *value*.
    /// 
    /// To name the name use [Invariant::get_table_name], this is done to make sure the given invariant name is valid.
    /// ```text
    /// | signature | value |
    /// +-----------+-------+
    /// | I?ABCd[v? | ##### |
    /// | I?ABCd[n? | ##### |
    /// | I?ABCd[^? | ##### |
    /// |          ...      |
    /// ```
    /// ## Exceptions
    /// Must panic when:
    /// * The given table name is already used
    async fn add_invariant_table<T: DBColumnTypes>(&self, invariant: &Invariant, column_type: T);



    
    /// Adds value to an already existing invariant table made using the [GraphDatabase::add_invariant_table()] function.
    /// 
    /// If the number of values to push is too big, consider using [GraphDatabase::add_values_from_buffer] instead, which is also using this method.
    /// ## Exceptions
    /// Must panic when:
    /// * The given table name doesn't not exists, because [GraphDatabase::add_invariant_table()] was not called before
    /// * The values to add are not valid.
    async fn add_values_to_inv_table<T>(&self, invariant: &Invariant, values: &Vec<T>);

    /// Fetch signatures from the dataset table.
    /// ## Args
    /// * `start_index` : The index of the table to start fetching the data at
    ///     * If the given value is `none`, the fetching will start a 0 
    /// * `end_index` : The index of the table to stop fetching the data at
    ///     * If the given value is `none`, the fetching will stop at the end of the table
    /// ## Exceptions
    /// Must panic when:
    /// * The given indexes are not valid
    async fn fetch_dataset_signatures(&self, start_index: Option<usize>, end_index: Option<usize>) -> Vec<String>;
    

    /// Reads line by line the given buffer and pushes it's content in the given datase.
    /// 
    /// In order to save memory, the method will use a vector to store the data read
    /// and after reaching a certain max capacity (being [BUFFER_VECTOR_MAX_SIZE]),
    /// will dump its content to the database using [GraphDatabase::fetch_dataset_signatures].
    async fn add_values_from_buffer(&self, reader: impl BufRead, invariant: &Invariant)
    {
        let mut signature_buffer : Vec<String> = Vec::new();
        for line in reader.lines() {
            if let Ok(sign) = line {
                println!("I just read: {sign}");
                signature_buffer.push(sign);
            }else {
                panic!("Could not read next buffer line");
            }
            // if we stored enough, we can push what we collected towards the given database
            if signature_buffer.len() == BUFFER_VECTOR_MAX_SIZE {
                self.add_values_to_inv_table(invariant, &signature_buffer).await; // add already stored signatures to the database
                signature_buffer.clear();   // free the *buffer*
            }
        }
        if signature_buffer.len() != 0
        {
            self.add_values_to_inv_table(invariant, &signature_buffer).await;  // add remaining values to the database
        }
    }

    
}

