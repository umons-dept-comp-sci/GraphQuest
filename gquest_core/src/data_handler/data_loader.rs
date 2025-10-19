use std::{
    fs::File,
    io::{stdin, BufRead, BufReader},
    process::{Child, ChildStdout, Command, Stdio},
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MethodError {
    #[error("Error while trying to open the given file: {0}")]
    File(String),
    #[error("Error while trying to create the geng query: {0}")]
    GengCreation(String),
    #[error("Error while trying to execute the geng query, are you sure geng is installed ?")]
    GengExecution,
}

/// Encapsulates a child process of a call to the `geng` program.
pub struct GengProcess {
    child: Child,
}

impl GengProcess {
    /// Creates and stores a `geng` process using a set of graph settings.
    pub fn call_geng(
        nb_of_vertices: u32,
        graph_settings: &String,
        edges_born: (Option<u32>, Option<u32>),
    ) -> Result<Self, MethodError> {
        let mut args: Vec<String> = vec![];

        if !graph_settings.is_empty() {
            if !graph_settings.starts_with("-") {
                // Checks if the '-' is missing
                args.push(format!("-{graph_settings}"));
            } else {
                args.push(graph_settings.clone());
            }
        }

        args.push(nb_of_vertices.to_string());

        match edges_born {
            (None, None) => (),
            (None, Some(nb)) => args.push(format!("0:{nb}")),
            (Some(nb), None) => args.push(format!("{nb}:0")),
            (Some(min), Some(max)) => {
                // min to max
                if max < min {
                    return Err(MethodError::GengCreation(format!("The maximum number({max}) of edges cannot be smaller than the given minimum({min})")));
                }
                args.push(format!("{min}:{max}"));
            }
        }

        let call_res = match Command::new("geng")
            .args(args)
            .arg("-q")
            .arg("-l") // Canonical form
            .stdout(Stdio::piped())
            .spawn()
        {
            Ok(res) => res,
            Err(_) => return Err(MethodError::GengExecution),
        };

        Ok(Self { child: call_res })
    }

    /// Get a buffer to the stdout of the child program.
    pub fn get_reader(mut self) -> BufReader<ChildStdout> {
        let stdout = self.child.stdout.take().expect("present");
        BufReader::new(stdout)
    }

    /// Wait and closes the process
    pub fn wait_close(mut self) {
        self.child.wait().unwrap();
    }
}

/// List of all methods available to read the data
pub enum Method {
    /// Read input from GengAPI
    GengAPI {
        /// The number of vertices of the graphs to add
        nb_of_vertices: u32,
        /// The graph parameters to give, they must follow the `geng` rules
        graph_settings: String,
        /// The edges boundaries of the graph to generate
        edges_bound: (Option<u32>, Option<u32>),
    },
    /// Read input from stdin
    Stdin,
    /// Read input from file using a given path
    File(String),
}

/// Wait for the stdin inputs of the user, reads and stores it in the given database.
pub fn read_pipe_signatures() -> impl BufRead {
    let stdin = stdin();
    stdin.lock()
}

/// Reads a file, located at the given path, by using a buffer, and stores it in the given database
pub fn read_file(path: &String) -> Result<impl BufRead, MethodError> {
    match File::open(path) {
        Ok(f) => Ok(BufReader::new(f)),
        Err(e) => Err(MethodError::File(e.to_string())),
    }
}
