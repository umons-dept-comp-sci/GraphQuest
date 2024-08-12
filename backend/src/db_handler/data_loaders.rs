use std::fs::File;
use std::process::{Command, Stdio};
use std::io::{stdin, BufReader, BufRead, Result, Lines};
use std::path::Path;

use db_handler::sqlite_handler::SqliteGraphDatabase;

use crate::db_handler;

use super::lib::{GraphDatabase, Invariant};  // Read stdout


/// The maximum capacity of the vectoe before pushing and flushing its content
const BUFFER_VECTOR_MAX_SIZE : usize = 2000;


/// Enums used to facilitate the types of graphs to generate using `geng` with each one translating into a setting of the command.  
pub enum GraphArgs {
    Connected,
    Biconnected,
    TriangleFree,
    FourCycleFree,
    FiveCycleFree,
    K4Free,
    Chordal,
    Split,
    Perfect,
    ClawFree,
    Bipartite,
    LowerBoundMinimumDegree(usize),
    UpperBoundMinimumDegree(usize),
}

/// Enums used to give more settings to `geng` 
pub enum GenSettings
{
    MinNumberOfEdges(usize),
    MaxNumberOfEdges(usize),
    // TODO Res/Mod ???
}

impl GraphArgs {
    /// Translates the given enum to the corresponding **geng** argument.
    /// Use `geng --help` for more detail
    fn to_arg(&self) -> String
    {
        let mut val: Option<usize> = None;
        let mut res = match self {
            GraphArgs::Connected => "c",
            GraphArgs::Biconnected => "C",
            GraphArgs::TriangleFree => "t",
            GraphArgs::FourCycleFree => "f",
            GraphArgs::FiveCycleFree => "p",
            GraphArgs::K4Free => "k",
            GraphArgs::Chordal => "T",
            GraphArgs::Split => "S",
            GraphArgs::Perfect => "P",
            GraphArgs::ClawFree => "F",
            GraphArgs::Bipartite => "b",
            GraphArgs::LowerBoundMinimumDegree(nb) => 
            {
                val = Some(*nb);
                "d"
            },
            GraphArgs::UpperBoundMinimumDegree(nb) => 
            {
                val = Some(*nb);
                "D"
            }
        }.to_string();

        if let Some(nb) = val {
            res.push_str(&nb.to_string());
        }
        res
    } 
}

/// Creates and stores the content of a `geng` query in the given database, using a set of graph settings.
/// The table name represents the name of the newly created table.
pub async fn load_table_with_geng(nb_of_vertices: usize, graph_settings: &[GraphArgs], db: &SqliteGraphDatabase, table_name: &str)
{
    // Concat all given args
    let args = {
        let mut r = "-".to_string();
        for graph_arg in graph_settings {
            r += graph_arg.to_arg().as_str();
        }
        r
    };
    
    
    // Call the geng command
    let mut call_res = Command::new("geng")
        .arg(args)
        .arg(nb_of_vertices.to_string())
        .arg("-q")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute the \"geng\" command");
    {
        let stdout = call_res.stdout.as_mut().unwrap();
        let stdout_reader = BufReader::new(stdout);

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



// TODO add load file method
#[warn(dead_code)]
fn read_lines<P>(filename: P) -> Result<Lines<BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(BufReader::new(file).lines())
}





pub enum Method 
{
    /// Read input from GengAPI
    GengAPI,
    /// Read input from stdin
    Stdin,
    /// Read input from file using a given path
    File(String)
} 

impl Method
{
    /// Read inpu
    pub async fn read_signatures<T: GraphDatabase>(&self, db:&T)
    {
        match self {
            Method::GengAPI => todo!(),
            Method::Stdin => read_pipe_signatures(db).await,
            Method::File(_) => todo!(),
        };
    }
}