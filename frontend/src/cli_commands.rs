use clap::{ArgAction, Args, Parser, Subcommand};

use clap_verbosity_flag::WarnLevel;
use std::fmt::Debug;

const DEFAULT_URL: &str = "sqlite://gquest.db";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct CliArg {
    #[command(subcommand)]
    pub cmd: Modes,
    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity<WarnLevel>,

}


#[derive(Subcommand, Debug, Clone)]
pub enum Modes {
    /// Initialise a project/database to work with
    Init {
        #[command(subcommand)]
        input_method : DatasetChoice,
        #[command(flatten)]
        path : DatabasePath,
    },
    /// Add graphs to an already existing project  
    Add {
        #[command(subcommand)]
        input_method : DatasetChoice,
        #[command(flatten)]
        path : DatabasePath,
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
pub struct DatabasePath {
    /// The url to the database to connect to
    #[clap(default_value = DEFAULT_URL)]
    pub url: String  
}



#[derive(Args, Debug, Clone)]
pub struct GengArgs {
    /// The order(s) of the graphs to generate
    #[clap(name("order|min:[max] order"))]  // TODO We should allow the user to exclude value from range
    pub order : String,
    /// The mininum and/or maximum number of edges of the graphs to generate
    #[clap(name("edges|min:[max] edges"), long("edges"), short('e'), value_delimiter = ':')]
    pub edges : Vec<Option<String>>,
    
    /// The parameters to give to geng in order to generate a graph of an order
    #[clap(long, short)]
    pub params : Option<String>,
}
   



#[derive(Subcommand, Debug, Clone)]
pub enum DatasetChoice {
    Geng {
        #[command(flatten)]
        args : GengArgs
    },
    Import {
        #[command(flatten)]
        args : ImportArgs
    }
}




#[derive(Debug, clap::Args, Clone)]
#[group(required = false, multiple = false)]        // This will stop the user from adding multiple arguments, meaning we do not have to check
pub struct ImportArgs {
    /// The file were the dataset is stored
    #[clap(short, long)]
    pub file: Option<String>,
    /// Read from the standart input (default) 
    #[clap(short, long, action=ArgAction::SetTrue, default_value="true")]
    pub read_stdin : bool,
}



#[derive(Args, Debug, Clone)]
struct  DatabaseInfoPath {
    /// The path or url to the database to access
    #[clap(short,long)]
    pub database_path_url: String,
}
