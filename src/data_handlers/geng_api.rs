use std::{borrow::Borrow, process::Command};


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
pub fn geng(nb_of_vertices: usize, graph_settings: &[GraphArgs]) -> String
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
    let call_res = Command::new("geng")
        .arg(args)
        .arg(nb_of_vertices.to_string())
        .arg("-q")
        .output()
        .expect("Failed to execute the \"geng\" command");

    // Return result as a string
     String::from_utf8(call_res.stdout).expect("Couldn't read the output as a String of the \"geng\" command")
    

    //let mut signature_vec: Vec<&str> = output
    //   .split("\n")
    //   .collect();
    //signature_vec.pop();
    //return signature_vec;

    //println!("Vector : {:?}", signature_vec);
}