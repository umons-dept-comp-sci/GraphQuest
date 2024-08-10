use clap::{ArgAction, Args, Parser, Subcommand};

const DEFAULT_URL: &str = "sqlite://gquest.db";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CliArg {
    #[command(subcommand)]
    cmd: Modes
}

#[derive(Subcommand, Debug, Clone)]
enum Modes {
    /// Initialise a project/database to work with
    Init {
        #[command(flatten)]
        path : DatabasePath
    },
    /// Add graphs to an already existing project  
    Add {
        #[command(flatten)]
        path : DatabasePath
    },
    /// Compute invariants from a dataset
    Compute {
        #[command(flatten)]
        path : DatabasePath,
        #[clap(default_value = "None")]
        dependencies_file: std::path::PathBuf,
        /// The list of programs to call
        #[clap(long, short, value_parser, num_args = 1.., value_delimiter = ' ')]
        programs: Vec<std::path::PathBuf>,
    },
    /// Delete a table from a dataset
    Delete {
        /// The name of the table/invariant to delete
        #[clap()]
        table_name: String, 
        #[command(flatten)]
        path: DatabasePath,
    },
    /// Send querries to a database
    Query {
        /// Do not output the result
        #[clap(short='u', long, action=ArgAction::SetTrue)]
        hide_output : bool,
        /// Do not saves the result
        #[clap(short, action=ArgAction::SetTrue)]
        do_not_save : bool,
        /// The query to ask the database
        #[clap()]
        formula : String,
        #[command(flatten)]
        path : DatabasePath,
    },
    /// Show a summary of a project
    Summary {
        #[command(flatten)]
        path : DatabasePath
    },
}


#[derive(Args, Debug, Clone)]
struct DatabasePath {
    /// The url to the database to connect to
    #[clap(default_value = DEFAULT_URL)]
    url: String  
}

#[derive(Args,  Debug, Clone)]
struct  DatabaseInfoPath {
    /// The path or url to the database to access
    database_path_url: String,
}

fn main() {
     
    let args = CliArg::parse();
    
    match args.cmd {
        Modes::Init { path } => todo!(),
        Modes::Add { path } => todo!(),
        Modes::Compute { path, dependencies_file, programs } => 
            println!("path: {:?} | dependencies: {:?} | programs : {:?}", path, dependencies_file, programs),
            
        Modes::Delete { path, table_name } => todo!(),
        Modes::Query { hide_output, do_not_save, formula, path } => todo!(),
        Modes::Summary { path } => todo!(),
    }
}
