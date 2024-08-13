use std::fs::File;
use std::process::{Command, Stdio};
use std::io::{stdin, BufReader, BufRead, Result, Lines};
use std::path::Path;
use std::ops::Range;

use super::super::db_handler::graph_database::*; 


/// Creates and stores the content of a `geng` query in the given database, using a set of graph settings.
/// The table name represents the name of the newly created table.
pub async fn load_table_with_geng<T: GraphDatabase>(db: &T,nb_of_vertices: u32, graph_settings: &String, edges_born: (Option<u32>, Option<u32>))
{
    let mut edges_args = String::new();
    if let (None, Some(nb)) = edges_born 
    {
        // nb or less
        edges_args = format!("0:{nb}");
    }
    if let (Some(nb), None) = edges_born
    {
        // nb or more
        edges_args = format!("{nb}:0");
    }
    if let (Some(min), Some(max)) = edges_born  {
        // min to max
        if max < min {
            panic!("The maximum number({max}) of edges cannot be smaller than the given minimum({min})");
        }
        edges_args = format!("{min}:{max}");
    }
    println!("geng {} {} {} -q", graph_settings, nb_of_vertices, edges_args);
    // Call the geng com<mand
    let mut call_res = match 
            Command::new("geng")
                .arg(format!("{} {} {} -q",  graph_settings, nb_of_vertices, edges_args))
                .stdout(Stdio::piped()) 
                .spawn()

         {
        Ok(res) => res,
        Err(e) => panic!("There was an error while trying to execute `geng`: {}", e),
    };
    
    
    {
        let stdout = call_res.stdout.as_mut().unwrap();
        let stdout_reader = BufReader::new(stdout);

        db.add_signatures_to_dataset_buffer(stdout_reader).await;
        //read_buffer(stdout_reader, db, table_name).await;
        //db.add_values_from_buffer(stdout_reader, &Invariant{name: "d"})
    }
    
        
    call_res.wait().unwrap();
}



/// Wait for the stdin inputs of the user, reads and stores it in the given database. 
pub async fn read_pipe_signatures<T: GraphDatabase>(db: &T)
{
    let stdin = stdin();
    db.add_signatures_to_dataset_buffer(stdin.lock()).await;
}


/// Reads a file by using a buffer, and stores it in the given database
async fn read_file<T:GraphDatabase> (db: &T, path: &String)
{
    let f = File::open(path).expect(format!("The given file path \"{path}\" is not valid").as_str());
    let bufread = BufReader::new(f);
    db.add_signatures_to_dataset_buffer(bufread).await;
}



/// List of all methods available to read the data
pub enum Method 
{
    /// Read input from GengAPI
    GengAPI{
        nb_of_vertices: u32,
        graph_settings: String,
        edges_born: (Option<u32>, Option<u32>)
    },
    /// Read input from stdin
    Stdin,
    /// Read input from file using a given path
    File(String)
} 

impl Method
{
    /// Read and add signatures to the given database using different input methods
    pub async fn read_signatures<T: GraphDatabase>(&self, db:&T)
    {
        match self 
        {
            Method::GengAPI { nb_of_vertices, graph_settings, edges_born } => load_table_with_geng(db, *nb_of_vertices, graph_settings, *edges_born).await,
            Method::Stdin => read_pipe_signatures(db).await,
            Method::File(path) => read_file(db, path).await,
        };
    }
}