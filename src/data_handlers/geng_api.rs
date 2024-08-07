use std::{process::{ChildStdout, Command, Stdio}, str::Lines};
use db_handler::*;
use std::{io::stdin, io::{BufReader, BufRead}};

use crate::db_handler;  // Read stdout

const BUFFER_VECTOR_MAX_SIZE : usize = 2000;


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
        // FIXME There is got to be a way cleaner way to do this
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

// geng -c 6 -q
pub async fn load_table_with_geng(nb_of_vertices: usize, graph_settings: &[GraphArgs], db: &GraphDatabase, table_name: &str) -> String
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
        let stdout_lines = stdout_reader.lines();

        let mut signature_buffer : Vec<String> = Vec::new();
        for line in stdout_lines {
            if let Ok(sign) = line {
                signature_buffer.push(sign);
            }else {
                panic!("damn");
            }

            // if we stored enough, we can push what we collected towards the given database
            if signature_buffer.len() == BUFFER_VECTOR_MAX_SIZE {
                //println!("Pushing what i collected");
                db.add_values(table_name, &signature_buffer).await;     // add already stored signatures to the database
                signature_buffer.clear();   // free the *buffer*
                //println!("Finished, moving on");
            }

        }
        // Push all signatures left
        if signature_buffer.len() != 0 {
            db.add_values(table_name, &signature_buffer).await;
        }
    }
    
        
    call_res.wait().unwrap();


    // Return result as a string
    //String::from_utf8(call_res.stdout).expect("Couldn't read the output as a String of the \"geng\" command");

    String::new()

    //println!("Vector : {:?}", signature_vec);
}


pub async fn read_buffer(lines: impl BufRead, db: &GraphDatabase, table_name: &str)
{
    let mut signature_buffer : Vec<String> = Vec::new();
    for line in lines.lines() {
        if let Ok(sign) = line {
            //println!("I just read: {sign}");
            signature_buffer.push(sign);
        }else {
            panic!("damn");
        }
        // if we stored enough, we can push what we collected towards the given database
        if signature_buffer.len() == BUFFER_VECTOR_MAX_SIZE {
            //println!("Pushing what i collected");
            db.add_values(table_name, &signature_buffer).await;     // add already stored signatures to the database
            signature_buffer.clear();   // free the *buffer*
            //println!("Finished, moving on");
        }
    }
}

pub async fn read_pipe_input(db: &GraphDatabase, table_name: &str)
{
    let stdin = stdin();
    read_buffer(stdin.lock(), db, table_name).await;

}