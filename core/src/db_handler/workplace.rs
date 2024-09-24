use std::marker::PhantomData;

use crate::{data_handler::{data_loaders::Method, invariant_handlers::{InvariantExecGroup, InvariantsExecManager}}, utils::{subject::Observer, table_handler::{QueryTable, QueryTableOptions}}};

use super::{db_errors::GraphDatabaseError, graph_database::*};




/// Encapsulation of a database connection that facilitates querries
pub struct Workspace<'a,  T: GraphDatabase<'a>> {
    /// The GraphDatabase used to modify the database state  
    db : T,
    _t : PhantomData<&'a T>
}



impl<'a, T: GraphDatabase<'a>> Workspace<'a, T> {
    /// Initialises a workplace
    /// 
    /// Will create everything needed by the program by using :
    /// * [GraphDatabase::create_graph_database] : to create the database
    /// * [GraphDatabase::create_dataset_table] : to create the dataset table
    /// * [GraphDatabase::create_meta_data_table] : to create the meta data table
    pub async fn init_workspace(db_url: &str) -> Result<Self, GraphDatabaseError>
    {
        // Init database
        let db = T::create_graph_database(db_url).await;
        if let Err(e) = db {
            Err(e)
        }
        else {
            let db = db.unwrap();
            // Init dataset table
            db.create_dataset_table().await.unwrap();
    
            // Init meta data table
            db.create_meta_data_table().await.unwrap();
    
            // Return the db connection encapsulated
            Ok(Workspace {
                db,
                _t : Default::default()
            })
        }
        
    }


    /// Connects to the workspace using the given url
    pub async fn connect_workspace(db_url: &str) -> Result<Self, GraphDatabaseError>
    {
        // Try to connect to the database
        let db = T::connect_graph_database(db_url).await;
        if let Err(e) = db {
            Err(e)
        }else {
            let db = db.unwrap();
            // Return the db connection encapsulated
            Ok(Workspace {
                db,
                _t : Default::default()
            })
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
    pub async fn execute_query(&self, query: &String, separator: Option<char>, output_path: Option<String>, stdout_opt: StdoutOptions, return_result: bool) -> Option<Vec<Vec<String>>>
    {
        self.db.join_all_invariant_tables().await.unwrap();
        
        self.db.execute_fetch_query(query, separator, output_path, stdout_opt, return_result).await.unwrap()
    }

    /// Gets a table that will summarize this workplace.
    /// 
    /// For example :
    /// ```b
    ///╭───┬────────────┬────────┬─────╮
    ///│ i │ Table Name │ Size   │ %   │
    ///├───┼────────────┼────────┼─────┤
    ///│ 0 │ Dataset    │ 288266 │ 100 │
    ///│ 1 │ size       │ 13598  │ 5   │
    ///│ 2 │ is_planar  │ 13598  │ 5   │
    ///│ 3 │ num_col    │ 3000   │ 1   │
    ///╰───┴────────────┴────────┴─────╯ 
    /// ```
    pub async fn summary(&self) -> QueryTable
    {
        let mut res = QueryTable::new(QueryTableOptions::Full);
        let dataset_len = self.get_dataset_length().await;
        // Create headers
        res.set_headers(vec!["Table Name".to_string(), "Size".to_string(), "%".to_string()]);
        // fetch all table names, if thet `get_all_table_names_query` is correct, the
        // only data contained in a line should be the name of a table
        let tables = self.db.execute_fetch_query(&self.db.get_all_tables_query(),
                                                 None, None, StdoutOptions::None,
                                                 true)
                                                 .await.unwrap().unwrap(); 
        // compute all relevent informations using those data
        for mut table in tables {
            // get size of the table
            let name = table[0].clone();
            // Skip the metadata table if it exists
            if name != METADATA_TABLE_NAME
            {
                let size = self.db.get_size_of_table(&name).await.expect(format!("Could not read the length of the table {}", name).as_str());
                
                // compute completion percentage
                let completion = ((size as f64 / dataset_len as f64) * 100 as f64).round();
                table.push(size.to_string());
                table.push(completion.to_string());
    
                res.push_line(table)
            }
        }
        res
    }
    


    pub async fn delete_table(&self, table_name: &String) -> Result<(), GraphDatabaseError>
    {
        self.db.delete_table(table_name).await
    }
}