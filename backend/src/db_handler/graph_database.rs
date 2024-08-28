
use std::fs::File;
use std::io::{stdout, Write};
use std::path::Path;
use std::process::ChildStdin;
use std::{io::BufRead, marker::PhantomData};


use crate::utils::subject::{Subject, Observer};
use crate::utils::table_handler::{QueryTable, QueryTableOptions};
use crate::utils::write_csv::CsvFile;

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
} 


/// Printing options for the query executions
pub enum OutputOptions
{
    /// Will not output anything
    None,
    /// Will output to the standart output and will separate the values using the given char 
    Stdout(char),
    /// Will print a table containing a summary of the result of the query
    PrettyTable
}


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
    async fn fetch_data(&self, start_index: Option<usize>, end_index: Option<usize>, inv_names_to_join: &Vec<String>, inputs: Vec<ChildStdin>) -> Result<(), GraphDatabaseError>;
    
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
                self.update_observator(byte_buffer, None);   
                notif_countdown = ITERATION_BEFORE_NOTIFY;  // Reset progression
                byte_buffer = 0;
            }
        }
        if signature_value_buffer.len() != 0
        {                
            self.update_observator(byte_buffer, None);   // Last notifications
            self.add_values_to_table(DATASET_TABLE_NAME, &signature_value_buffer).await;  // add remaining values to the database
        }
    }

    /// Push received data to the invariants table
    async fn push_data_from_buffer(&self, inv_exec: &InvariantsExecutable, index_in_group: usize, reader: impl BufRead)
    {
        // we use a vector of vec in order to store each (signature, value) for each computed invariants
        let mut signature_value_buffer : Vec<Vec<(String, String)>> = Vec::new();
        
        for _ in 0..inv_exec.names.len() {
            signature_value_buffer.push(vec![]);
        }
        // if the invariants were not already added
        for inv in &inv_exec.names {
            self.init_invariant(inv).await;
        }


        for line in reader.lines() {
            
            if let Ok(sign_value) = line {
                //println!("Reading : {sign_value}");
                if sign_value != "\n" && sign_value != "" {

                    let values: Vec<&str> = sign_value.split_ascii_whitespace().collect(); // TODO We could change the split char with something else
                    if values.len() != inv_exec.names.len() + 1 {
                        panic!("Expected {} values from the executable \"{}\" but only read {}: {:?}", inv_exec.names.len() + 1, inv_exec.exec_path, values.len(), sign_value);
                    }
                    // Store data in the
                    for i in 0..values.len()-1 {
                        // Add a signature and the value to the corresponding table
                        signature_value_buffer[i].push((values[0].to_string(), values[i+1].to_string()));   
                    }

                    //self.tick_observator(None);
                }
            }else {
                panic!("Could not read next buffer line");
            }
            // if we stored enough, we can push what we collected towards the given database
            if signature_value_buffer.len() == BUFFER_VECTOR_MAX_SIZE {
                // update observator
                self.update_observator(1 as u64, Some(index_in_group));
                for (i, inv) in inv_exec.names.iter().enumerate() {

                    self.add_values_to_table(&InvariantsExecutable::get_table_name_from_string(inv), &signature_value_buffer[i]).await;
                    signature_value_buffer[i].clear();   // free the *buffer*
                }
            }
            
        }
        if signature_value_buffer.len() != 0
        {                
            // update obs
            self.update_observator(signature_value_buffer[0].len() as u64, Some(index_in_group));   // Last notifications
            for (i, inv) in inv_exec.names.iter().enumerate() {

                self.add_values_to_table(&InvariantsExecutable::get_table_name_from_string(inv), &signature_value_buffer[i]).await;
                signature_value_buffer[i].clear();   // free the *buffer*
            }
        }
    }

    /// Closes the connection with the database
    async fn close_connection(self);

    /// Executes the query to the database and fetches the output in a human readable way
    async fn execute_fetch_query<'b>(&self, query: &String, output_file: Option<CsvFile>, query_table: Option<QueryTable>) -> Result<String, GraphDatabaseError>
    {    

        let write_lines = move |column_names: Vec<String>, lines: Vec<Vec<String>>|
        {
            if let Some(mut file) = output_file
            {
                file.write_lines_to_file(column_names.clone(), lines.clone()).expect("Could not write to result file");
            }

            if let Some(mut table) = query_table
            {
                if !table.headers_added() {
                    table.set_headers(column_names);
                    for line in lines {
                        
                        table.push_line(line);
                    }
                }
                println!("fucl");
                //println!("{}", table.as_string());
            }
                
        };


        let stdout = stdout(); // get the global stdout entity
        let mut handle = stdout.lock();
        
        
        //Self::test(x).await;
        self.execute_query(query, write_lines).await;

        

        Ok(String::from(":)"))
    }

    /// Executes a query and calls the given functions
    async fn execute_query(&self, query: &String, f: impl FnOnce(Vec<String>, Vec<Vec<String>>));

    async fn test(mut f: impl FnMut(String))
    {
        f(String::from("Verry cool lambda axel"));
    }

    //_________________________________INVARIANTS_______________________________________________________________________________

    /// Creates the given invariant table and adds it to the meta data table,
    /// if it wasn't already added
    async fn init_invariant(&self, inv: &String)
    {
        if !self.inv_already_added(inv).await
        {
            // Create table
            // TODO, by default we use an int but we do need to check the return type of this invariant
            self.create_invariant_table(inv).await;
            // Add Metadata line
            self.update_meta_data(&InvariantsExecutable::get_table_name_from_string(inv), 0).await;
        }
    }

    /// Checks if the invariant was already added to the database
    async fn inv_already_added(&self, inv: &String) -> bool;
    
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
    async fn create_invariant_table(&self, invariant: &String);


    /// Simply returns the length of the table with the given name
    /// ## Exceptions
    /// Must return a TableNotFoundError if the given table is not found
    async fn get_size_of_table(&self, name: &str) -> Result<usize, GraphDatabaseError>;
    

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

    /// Adds the given dataset to the workplace
    pub async fn add_dataset(&mut self, method: Method, temp_obs: &'a dyn Observer )
    {
        self.db.set_graph_db_observer(temp_obs);
        method.read_signatures(&self.db).await;

        self.db.remove_graph_db_observer();
    }



    // Returns the length of the dataset
    pub async fn get_dataset_length(&self) -> usize
    {
        self.db.get_size_of_table(DATASET_TABLE_NAME).await.unwrap()
    }

    

    /// Properly closes the worspace
    pub async fn close_workspace(self)
    {
        self.db.close_connection().await;
    }
    
    /// Gets the different groups to later execute using the given [InvariantExecManager], and calls [InvariantExecGroup::fetch_progress_info] for each one 
    pub async fn prepare_groups(&self, exec: InvariantsExecManager, process_available: usize) -> Vec<InvariantExecGroup>
    {
        let mut groups = exec.group_process_executions(process_available);
        for group in &mut groups {
            group.fetch_progress_info(&self.db).await;
        }
        groups
    }

    /// Executes the given invariant groups and adds (if given) an observer to the database
    pub async fn execute_group(&mut self, mut group: InvariantExecGroup, temp_obs: Option<&'a dyn Observer>)
    {
        if let Some(obs) = temp_obs {
            self.db.set_graph_db_observer(obs);
        }
        group.exec_invariants(&self.db).await;
    }


    /// Executes the given query to the database
    pub async fn execute_query(&self, query: &String, separator: Option<char>, output_file: Option<String>, output_opt: OutputOptions)
    {
        let mut f: CsvFile = CsvFile::new(&output_file.unwrap(), separator).unwrap();

        let mut table_query = QueryTable::new(QueryTableOptions::Partial { first_rows_count: 5, last_rows_count: 0 });
        table_query.set_headers(vec![String::from("Column1"), String::from("Column2"), String::from("Column3")]);
        table_query.push_line(vec![String::from("Value1"), String::from("Value2"), String::from("Value3")]);
        
        
        //println!("{}", table_query.as_string());

        self.db.execute_fetch_query(query, Some(f),  Some(table_query)).await;
    }
    
}