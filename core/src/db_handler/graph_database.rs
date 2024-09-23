use std::io::{stdout, Lines, Write};
use std::process::ChildStdin;
use std::{io::BufRead, marker::PhantomData};


use log::debug;

use crate::utils::subject::{Subject, Observer};
use crate::utils::table_handler::{QueryTable, QueryTableOptions};
use crate::utils::write_csv::{as_line, CsvFile};

use super::super::data_handler::data_loaders::*;
use super::super::data_handler::invariant_handlers::*;
use super::db_errors::*;

/// The maximum capacity of the vector before pushing and flushing its content
pub const BUFFER_VECTOR_MAX_SIZE : usize = 2000;
/// The name of the first created table of the dataset containing the initial dataset
pub const DATASET_TABLE_NAME : &str = "Dataset";
/// The column name of the primary key of the dataset
pub const PK_NAME : &str = "signature";
/// The name of the second column of the dataset 
pub const DATASET_VALUE_NAME : &str = "vertices";
/// The name of the the metadata table
pub const METADATA_TABLE_NAME : &str = "Metadata";
/// The name of the primary key of the metadata table
pub const METADATA_PK_NAME : &str = "table_name";
/// The name of the second column of the metadata table
pub const METADATA_VALUE_NAME : &str = "stopped_at";
// The name of the column in an invariant table where the values are stored 
//pub const INVARIANT_COLUMN_NAME : &str = "value";
/// The name of the table that has all the data
pub const FULL_TABLE_NAME: &str = "AllInv";


/// The maximum size of a signature to store in the dataset
pub const SIGNATURE_MAX_SIZE : usize = 250;
/// The maximum size of a table name in the dataset
pub const TABLE_NAME_MAX_SIZE : usize = 250;
/// The speed at which the observator will be notified (if any present on db)
const ITERATION_BEFORE_NOTIFY : u8 = 10;



pub enum ColumnType
{
    String {
        max_size: Option<usize>,
        default_value: Option<String>
    },
    Integer{
        default_value: Option<usize>
    },
    Boolean{
        default_value: Option<bool>
    }
}


/// Traits used to offer a selection of different possible data types for a database.
pub trait DBColumnTypes
{
    /// Translates the given struct as a string which can be used in a query in order to 
    /// represent a type from a database
    fn translate(&self) -> String;
} 



/// Printing options for the query executions
#[derive(Debug)]
pub enum StdoutOptions
{
    /// Will not output anything
    None,
    /// Will output to the standart output and will separate the values using the given char and will add a '\n' char to each end of line
    Stdout(char),
    /// Will print a table containing a summary of the result of the query
    PrettyTable(QueryTableOptions)
}


/// The GraphDatabase trait is used to facilitate the communication with databases for the user.
pub trait GraphDatabase<'a> : Subject<'a> + Clone 
{
    
    /// Creates the database that will be storing the project.
    /// 
    /// Returns a struct implementing the [GraphDatabase] trait.
    /// ## Exceptions
    /// Returns :
    /// * [GraphDatabaseError::DatabaseAlreadyCreated] if the database was already created
    /// * [GraphDatabaseError::UnknownError] if an unknown error was uncountered when trying to create it
    async fn create_graph_database(db_url: &str) -> Result<Self, GraphDatabaseError>;


    /// Connects to the given database
    /// 
    /// ## Exceptions
    /// Returns a:
    /// *   [GraphDatabaseError::DatabaseNotFound] if the url is not valid (no database was found using the given url)
    async fn connect_graph_database(db_url: &str) -> Result<Self, GraphDatabaseError>;

    /// Closes the connection with the database
    async fn close_connection(self);
    
    //__________________________ GETTERS _____________________________________________________________

    /// Gets a query that returns all the names from the database
    fn get_all_tables_query(&self) -> String;

    /// Returns the query that can be used to create a table called "table_name",
    /// with two columns :
    /// * "*pk_name*": The primary key column (must be able to contain strings of [SIGNATURE_MAX_SIZE] size)
    /// * "*value_name*": The column that will store values
    fn get_create_table_query(table_name: &str, pk_name: &str, value_name: &str, value_type: ColumnType) -> String;
        
    /// Returns the query that can be used to delete a table from the dataset
    fn get_delete_table_query(table_name: &str) -> String;
    
    
    /// Returns the query that can be used to insert all the given data into a table called "*table_name*"
    fn get_insert_into_query(table_name: &str, signatures_values: &Vec<(String, String)>) -> String;
        
    /// Get a query that can be used to join all the given tables using a common column
    fn get_join_table_query(table_names: Vec<String>, common_column_name: &str) -> String;
        
    /// Get a query that can be used to retrieve all row from the given table and column
    fn get_all_rows_from_table_column(table_name: &str, column_name: &str) -> String;
    
    /// Get a query that can be used to select a batch from a given table
    /// ## Args
    /// * `start_index` : The index of the table to start fetching the data at
    ///     * If the given value is `none`, the fetching will start a 0 
    /// * `limit` : The limit on the number of value to fetch
    ///     * If the given value is `none`, the fetching will be stop at the end of the table
    fn get_select_batch_from(from_table: String, start_index: Option<usize>, limit: Option<usize>) -> String;
    

    //__________________________ WRITING DATA _____________________________________________________________

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
                self.add_values_to_table(DATASET_TABLE_NAME, &signature_value_buffer).await.expect("Could not add value to table"); // add already stored signatures to the database
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
            self.add_values_to_table(DATASET_TABLE_NAME, &signature_value_buffer).await.expect("Could not add value to table");  // add remaining values to the database
        }
    }

    /// Push received data to the invariant tables
    async fn push_data_from_buffer<T: BufRead>(&self, mut max_to_read: usize, inv_exec: &InvariantsExecutable, reader: & mut Lines<T>)
    {
        // we use a vector of vec in order to store each (signature, value) for each computed invariants
        let mut signature_value_buffer : Vec<Vec<(String, String)>> = Vec::new();
        
        for _ in 0..inv_exec.names.len() {
            signature_value_buffer.push(vec![]);
        }
        // if the invariants were not already added
        for inv in &inv_exec.names {
            self.init_invariant(inv).await.unwrap();
        }
        let mut notif_countdown = ITERATION_BEFORE_NOTIFY;
        for line in reader {
            if let Ok(sign_value) = line {
                
                max_to_read -= 1;
                
                if sign_value != "\n" && sign_value != "" {

                    let values: Vec<&str> = sign_value.split_ascii_whitespace().collect(); // TODO We could change the split char with something else
                    if values.len() != inv_exec.names.len() + 1 {
                        panic!("Expected {} values from the executable \"{}\" but read {}: {:?}", inv_exec.names.len() + 1, inv_exec.exec_path, values.len(), sign_value);
                    }
                    // Store data in the buffer
                    for i in 0..values.len()-1 {
                        // Add a signature and the value to the corresponding table
                        signature_value_buffer[i].push((values[0].to_string(), values[i+1].to_string()));   
                    }
                }
            }else {
                panic!("Could not read next buffer line");
            }
            self.tick_observator();
            // update observator
            notif_countdown -= 1;   // Update countdown
            // If it is time to notify the observor
            if notif_countdown == 0 {
                self.update_observator(ITERATION_BEFORE_NOTIFY as u64);
                notif_countdown = ITERATION_BEFORE_NOTIFY;  // Reset progression
            }
            // if we stored enough, we can push what we collected towards the given database
            if signature_value_buffer.len() == BUFFER_VECTOR_MAX_SIZE {
                for (i, inv) in inv_exec.names.iter().enumerate() {

                    self.add_values_to_table(&InvariantsExecutable::get_table_name_from_string(inv), &signature_value_buffer[i]).await.expect("Could not add value to table");
                    signature_value_buffer[i].clear();   // free the *buffer*
                }
            }
            
            if max_to_read == 0 {
                if signature_value_buffer.len() != 0
                {                
                    for (i, inv) in inv_exec.names.iter().enumerate() {

                        self.add_values_to_table(&InvariantsExecutable::get_table_name_from_string(inv), &signature_value_buffer[i]).await.expect("Could not add value to table");
                        signature_value_buffer[i].clear();   // free the *buffer*
                    }
                }
                return;
            }
        }
        debug!("buffer closed");
        if signature_value_buffer.len() != 0
        {            
            // update obs
            self.update_observator(signature_value_buffer[0].len() as u64);   // Last notifications
            for (i, inv) in inv_exec.names.iter().enumerate() {

                self.add_values_to_table(&InvariantsExecutable::get_table_name_from_string(inv), &signature_value_buffer[i]).await.expect("Could not add value to table");
                signature_value_buffer[i].clear();   // free the *buffer*
            }
        }
    }

    /// Adds value to the dataset table.
    /// 
    /// If the number of values to push is too big, consider using [GraphDatabase::add_signatures_to_dataset_buffer()] instead, which is also using this method.
    /// ## Exceptions
    /// Can return an [GraphDatabaseError] when:
    /// * The given table name doesn't not exists, because [GraphDatabase::create_dataset_table()] was not called before
    /// * The values to add are not valid.
    /// * The values break the primary key rule (ex. a signature is already inside the dataset)
    async fn add_values_to_table(&self, table_name: &str, signatures_values: &Vec<(String, String)>) -> Result<(), GraphDatabaseError>
    {
        // Get query
        let query = Self::get_insert_into_query(table_name, signatures_values);

        // Exec query
        self.execute_query_no_return(&query).await
    }

    /// Fetches signatures from the dataset table and writes them into the given stdins
    /// ## Args
    /// * `start_index` : The index of the table to start fetching the data at
    ///     * If the given value is `none`, the fetching will start a 0 
    /// * `limit` : The limit on the number of value to fetch
    ///     * If the given value is `none`, the fetching will be stop at the end of the table
    /// * `inv_names_to_join` : The names of all invariants columns to join when fetching the data
    /// ## Exceptions
    /// Must panic when:
    /// * The given indexes are not valid
    async fn fetch_write_data(&self, start_index: Option<usize>, limit: Option<usize>, mut dependencies_to_join: Vec<String>, inputs: &Vec<ChildStdin>) -> Result<(), GraphDatabaseError>
    {
        // Get join query
        let join_query : String = {
            if dependencies_to_join.len() == 0 {
                Self::get_all_rows_from_table_column(DATASET_TABLE_NAME, PK_NAME)
            }
            else {
                Self::get_join_table_query(dependencies_to_join, PK_NAME)
            }
        };
 
        let limit_offset_query = Self::get_select_batch_from(join_query, start_index, limit);


        let write_lines = |_: Vec<String>, lines: Vec<Vec<String>>|
        {
            for line in lines  {
                let mut str = line.join(" ");
                str.push('\n');
                for mut input in inputs {
                    input.write(str.as_bytes()).unwrap();
                    input.flush().expect("Could not flush stdin of process");   
                }
            }
        };

        let box_fn = Box::new(write_lines);
        
        // EXECUTE QUERY
        let que_res = self.execute_query(&limit_offset_query, box_fn).await;
        
        if let Err(e) = que_res {
            return Err(e);
        }

        Ok(())
    }

    
    //__________________________ READ DATA _____________________________________________________________

    /// Executes the query to the database and fetches the output in a human readable way
    async fn execute_fetch_query<'b>(&self, query: &String, separator: Option<char>, output_path: Option<String>, stdout_opt: StdoutOptions, return_result: bool) -> Result<Option<Vec<Vec<String>>>, GraphDatabaseError>
    {  
        
        let mut saved_output: Vec<Vec<String>> = vec![];
        
        let mut query_table: Option<QueryTable> = None;
        let mut output_file:  Option<CsvFile> = {
            match output_path {
                Some(path) => Some(CsvFile::new(&path, separator).expect("Could not create the output path")),
                None => None,
            }
        };
        let mut write_to_stdout = (false, ',', false);
        match stdout_opt {
            StdoutOptions::None => (),
            StdoutOptions::Stdout(opt) => write_to_stdout = (true, opt, false),
            StdoutOptions::PrettyTable(opt) => query_table = Some(QueryTable::new(opt)),
        }

        // Pointers used to call those struct inside the *lambda* function without losing ownerships
        let table_ref = &mut query_table;
        let output_ref = &mut output_file;
        let saved_output_ref = &mut saved_output;

        let stdout = stdout(); // get the global stdout entity
        let mut handle = stdout.lock();
        
        
        let write_lines = |column_names: Vec<String>, lines: Vec<Vec<String>>|
        {
            // Add to file if exists
            if write_to_stdout.0 {
                if !write_to_stdout.2{
                    write!(handle, "{}", as_line(&column_names, write_to_stdout.1)).expect("Could not write column names to stdout");
                    write_to_stdout.2 = true;
                }
                for line in &lines {
                    write!(handle, "{}", as_line(line, write_to_stdout.1)).expect("Could not write column names to stdout");
                }
            }
            if let Some(file) = output_ref{
               file.write_lines_to_file(column_names.clone(), lines.clone()).expect("Could not write to result file");
            }
            
            // Add to table if exists
            if let Some(table) = table_ref{
                if !table.headers_added() {
                    table.set_headers(column_names);
                }
                for line in &lines {
                    table.push_line(line.clone());
                }
            }

            if return_result {
                for line in lines {
                    saved_output_ref.push(line)
                }
            }
        };

        let box_fn = Box::new(write_lines);
        
        
        self.execute_query(query, box_fn).await.unwrap();
        // If a table was given, print it
        if let Some(table) = query_table {
            
            writeln!(handle, "{}",table.as_string()).unwrap();
        }

        Ok(
            if return_result {
               Some(saved_output) 
            }else {
                None
            }
        )
    }

    //__________________________ EXECUTE QUERY _____________________________________________________________

    /// Executes a query and calls the given function which takes two parameters
    /// * A `Vec<String>` which represents the headers of the columns from the query result
    /// * A `Vec<Vec<String>>` which represents the lines from the query result
    /// This function is called multiple times in order to not have to store too much data 
    async fn execute_query(&self, query: &String, save_data_fn: impl FnMut(Vec<String>, Vec<Vec<String>>)) -> Result<(), GraphDatabaseError>;
    

    /// Execute a query but does not look at the return value
    /// ## Exceptions
    /// Returns:
    /// * [GraphDatabaseError::QueryError] when the query failed 
    async fn execute_query_no_return(&self, query: &String) -> Result<(),GraphDatabaseError>;


    //__________________________ INVARIANTS HANDLING _____________________________________________________________

    /// Creates the given invariant table and adds it to the meta data table,
    /// if it wasn't already added
    async fn init_invariant(&self, inv: &String) -> Result<(), GraphDatabaseError>
    {
        match self.inv_already_added(inv).await {
            Ok(b) => {
                if !b{
                 // Create table
                self.create_invariant_table(inv).await.unwrap();
                // Add Metadata line
                self.update_meta_data(&InvariantsExecutable::get_table_name_from_string(inv), 0).await.unwrap();
                }
            },
            Err(e) => {
                return Err(e);
            },
        }
        return Ok(());
    }

    /// Checks if the invariant was already added to the database
    async fn inv_already_added(&self, inv: &String) -> Result<bool, GraphDatabaseError>;
    

    //__________________________ GET INFO FROM DB _____________________________________________________________

    /// Simply returns the length of the table with the given name
    /// ## Exceptions
    /// Must return a TableNotFoundError if the given table is not found
    async fn get_size_of_table(&self, name: &str) -> Result<usize, GraphDatabaseError>;
    

    /// Gets the minimum size between all given tables and the dataset table 
    /// ## Exceptions
    /// Will return [GraphDatabaseError::TableNotFoundError] if one of the given table doesn't not exists
    async fn get_min_dependency_size(&self, tables: &Vec<String>) -> Result<usize, GraphDatabaseError>
    {
        // The starting min size is obviously the quantity of data stored inside the dataset
        let mut min_size = self.get_size_of_table(DATASET_TABLE_NAME).await.unwrap();
        
        // Get minimum
        let mut table_size;
        for table in tables {
            
            table_size = self.get_size_of_table(table).await;
            if let Err(e) = table_size {
                return Err(e);
            }
            let table_size = table_size.unwrap();
            if table_size < min_size {
                min_size = table_size;
            }
        }
        Ok(min_size)
    }

    // FIXME, this can be improved greatly using the METADATA table
    /// Joins all tables from the dataset using the [PK_NAME] column 
    /// ## Exceptions
    /// Returns a [GraphDatabaseError] if an error was encountered
    async fn join_all_invariant_tables(&self) -> Result<(), GraphDatabaseError>
    {
        // Get all table names
        let table_names = self.get_all_table_names().await;

        if let Err(e) = table_names{
            return Err(e)
        } 
        let mut table_names = table_names.unwrap();
        table_names.insert(0, DATASET_TABLE_NAME.to_string());

        // Get the minimum table size
        let min_full_size = self.get_min_dependency_size(&table_names).await;

        if let Err(e) = min_full_size {
            return Err(e);
        }
        let dataset_size = min_full_size.unwrap();
        
        // If table created
 
        match self.get_size_of_table(FULL_TABLE_NAME).await {
            Ok(n) => {
                // And the table is already fully made
                if n == dataset_size {
                    return Ok(());
                }
            },
            Err(e) => {
                if let GraphDatabaseError::TableNotFoundError { table_name: _ } = e  {
                    // pass   
                }
                else {
                    return Err(e);
                }
            },
        }
        
        // Delete previously made table
        if let Err(e) = self.delete_table(FULL_TABLE_NAME, true).await{
            if let GraphDatabaseError::TableNotFoundError { table_name: _ } = e {
                // pass
            }
            else{
                return Err(e);
            }
        }
        self.join_save_tables(FULL_TABLE_NAME, table_names, PK_NAME).await
        
    }
        
    /// Returns all the table names except for:
    /// * The [METADATA_TABLE_NAME] table
    /// * The [DATASET_TABLE_NAME] table
    /// * The [FULL_TABLE_NAME] data table
    /// ## Exceptions
    /// Returns a [GraphDatabaseError] if an error was encountered
    async fn get_all_table_names(&self) -> Result<Vec<String>, GraphDatabaseError> {
        let query = String::from("SELECT name FROM sqlite_master WHERE type='table';");
        let que_res = self.execute_fetch_query(&query, None, None, StdoutOptions::None, true).await;
        if let Err(e) = que_res {
            return Err(e);
        }
        else {
            let mut tables: Vec<String> = vec![];
            let que_res = que_res.unwrap().unwrap();
            for lines in que_res {
                for value in lines  {
                    if !(value == METADATA_TABLE_NAME || value == DATASET_TABLE_NAME || value == FULL_TABLE_NAME)
                    {
                        tables.push(value);
                    }
                }
            }
            Ok(tables)
        }
    }

    //__________________________ CREATE TABLE DB _____________________________________________________________


    /// Adds a dataset table to the database that will be used to store all initial signatures.
    /// The database table has two columns.
    /// ## Exemple of table
    /// ```text
    /// InitDataset -> | signature | vertices |
    ///                +-----------+----------+
    ///                | I?ABCd[v? |    7     |
    ///                | I?ABCd[n? |    7     |
    ///                | I?ABCd[^? |    7     |
    ///                |          ...         |
    /// ```
    /// ## Exceptions
    /// Returns: 
    /// * [GraphDatabaseError] if something went wrong with the query
    async fn create_dataset_table(&self) -> Result<(), GraphDatabaseError>
    {
        // Get query
        let query = Self::get_create_table_query(DATASET_TABLE_NAME, PK_NAME, DATASET_VALUE_NAME, ColumnType::String { max_size: Some(SIGNATURE_MAX_SIZE), default_value: None });
        // Exec query
        self.execute_query_no_return(&query).await
    }
    

    /// Adds a metadata table to the database that will be used to not recompute the [FULL_TABLE_NAME] table.
    /// 
    /// ## Exemple of table
    /// ```text
    /// Metadata -> | table_name  | stopped_at |
    ///             |-------------|------------|
    ///             | InitDataset | 1500       |
    ///             | Euler       | 753        |
    ///             |            ...           |
    /// ```
    /// ## Exceptions
    /// Returns: 
    /// * [GraphDatabaseError] if something went wrong with the query
    async fn create_meta_data_table(&self)  -> Result<(), GraphDatabaseError>
    {
        // Get query
        let query = Self::get_create_table_query(METADATA_TABLE_NAME, "table_name", METADATA_VALUE_NAME, ColumnType::Integer { default_value: Some(0) });

        // Exec Query
        self.execute_query_no_return(&query).await
    }

    /// Adds a table to the database to later store the value of an invariant for each graph of the database.
    /// 
    /// An invariant table has two columns named [DATASET_PK_NAME] (which is the primary key) and the other one has the same name of the table.
    /// 
    /// It uses [Invariant::get_table_name] to get the table name, this is done to make sure the given invariant name is valid for a database table.
    /// 
    /// ## Exemple of table
    /// ```text
    /// Chromatic -> | signature | Chromatic_Number |
    /// _Number      +-----------+------------------+
    ///              | I?ABCd[v? | #####            |
    ///              | I?ABCd[n? | #####            |
    ///              | I?ABCd[^? | #####            |
    ///              |          ...                 |
    /// ```
    /// ## Exceptions
    /// Can return an [GraphDatabaseError] when:
    /// * The given table name is already used
    async fn create_invariant_table(&self, inv: &String) -> Result<(), GraphDatabaseError>
    {
        let inv_name = InvariantsExecutable::get_table_name_from_string(inv);

        let query = Self::get_create_table_query(&inv_name, PK_NAME, &inv_name, ColumnType::Integer { default_value: None } );
        
        let res = self.execute_query_no_return(&query).await;
        if let Err(_) = res {
            return Err(GraphDatabaseError::TableAlreadyCreatedError { table_name: inv_name });
        }
        Ok(())
    }
    
    /// Deletes the given table
    /// ## Exceptions
    /// returns: 
    /// * A [GraphDatabaseError::TableNotFoundError] when the given table is not present
    /// * A [GraphDatabaseError::ForbiddenActionError] when the given table cannot be deleted
    /// * A [GraphDatabaseError] in general, if something else went wrong
    async fn delete_table(&self, table_name: &str, can_delete_full: bool) -> Result<(), GraphDatabaseError>
    {
        let low_table_name = table_name.to_lowercase();
        // Check if the table is not critical
        if low_table_name == DATASET_TABLE_NAME.to_lowercase() || low_table_name == DATASET_TABLE_NAME.to_lowercase() || (low_table_name == FULL_TABLE_NAME.to_lowercase() && !can_delete_full)  {
            return Err(GraphDatabaseError::ForbiddenActionError { action: format!("Tried to delete the table {}", table_name)})
        }

        // check if the table exists
        if let Err(e) = self.get_size_of_table(table_name).await {
            return Err(e);
        }
        
        let query = Self::get_delete_table_query(table_name);
        
        self.execute_query_no_return(&query).await
    }


    /// Joins all the given tables and creates a new table with the given name.
    /// At least one table name must be provided.
    /// ## Exceptions
    /// Returns a [GraphDatabaseError] if an error was encountered
    async fn join_save_tables(&self, new_table_name: &str, table_names: Vec<String>, common_column_name: &str) -> Result<(), GraphDatabaseError>;


    // TODO this has to be modified in order to use it later for the [FULL_TABLE_NAME] Table creation
    async fn update_meta_data(&self, changed_table_name: &str, added_values: usize) -> Result<(), GraphDatabaseError>
    {
        // Get query
        let query = Self::get_insert_into_query(METADATA_TABLE_NAME, &vec![(changed_table_name.to_string(), added_values.to_string())]);

        // Exec query
        self.execute_query_no_return(&query).await
    }

    

    
    
}
