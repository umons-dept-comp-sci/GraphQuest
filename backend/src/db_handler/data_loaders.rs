use std::fs::File;
use std::process::{Command, Stdio};
use std::io::{stdin, BufReader, BufRead, Result, Lines};
use std::path::Path;

use db_handler::sqlite_handler::GraphDatabase;

use crate::db_handler;  // Read stdout


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

/// Creates and stores the content of a `geng` query in the given database, using a set of graph settings.
/// The table name represents the name of the newly created table.
pub async fn load_table_with_geng(nb_of_vertices: usize, graph_settings: &[GraphArgs], db: &GraphDatabase, table_name: &str)
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

        read_buffer(stdout_reader, db, table_name).await;
    }
    
        
    call_res.wait().unwrap();
}

/// Reads line by line the given buffer and pushes it's content in the given datase. 
/// The table name represents the name of the newly created table.
/// 
/// In order to not crash, the method will use a vector to store the data read and after reaching a certain max capacity (being [BUFFER_VECTOR_MAX_SIZE]), will dump its content to the database. 
pub async fn read_buffer(reader: impl BufRead, db: &GraphDatabase, table_name: &str) -> Vec<String>
{
    let mut signature_buffer : Vec<String> = Vec::new();
    for line in reader.lines() {
        if let Ok(sign) = line {
            println!("I just read: {sign}");
            signature_buffer.push(sign);
        }else {
            panic!("damn");
        }
        // if we stored enough, we can push what we collected towards the given database
        if signature_buffer.len() == BUFFER_VECTOR_MAX_SIZE {
            db.add_values(table_name, &signature_buffer).await;     // add already stored signatures to the database
            signature_buffer.clear();   // free the *buffer*
        }
    }
    signature_buffer
}

/// Wait for the stdin inputs of the user, reads and stores it in the given database. 
/// The table name represents the name of the newly created table.
pub async fn read_pipe_input(db: &GraphDatabase, table_name: &str)
{
    let stdin = stdin();
    read_buffer(stdin.lock(), db, table_name).await;
}



// TODO add load file method
#[warn(dead_code)]
fn read_lines<P>(filename: P) -> Result<Lines<BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(BufReader::new(file).lines())
}




#[cfg(test)]
mod tests {
    use super::*;   // Import all

    //#[sqlx::test]
    async fn create_db_geng_test(pool: sqlx::SqlitePool) -> sqlx::Result<()> {
        let db = GraphDatabase::create_graph_database(pool);
        load_table_with_geng(8, &[GraphArgs::Connected], &db, "test_table1").await;
        
        db.create_graph_table("test_table").await;
        let query_res = db.query_return_string("SELECT COUNT(DISTINCT *) FROM test_table1;").await.unwrap();
        assert_eq!(Some("11117".to_string()), query_res);  // check if table was indeed created
        Ok(())
    }


}