use clap::{ArgAction, Args, Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};

const DEFAULT_URL: &str = "sqlite://gquest.db";


#[derive(Parser)]
#[command(
    author("Axel Foucart"),
    version,
    about("gquest: Developped by Axel Foucart at Algorithm Lab, UMONS-2024-2025")
)]
pub struct CliArg {
    #[command(subcommand)]
    pub cmd: Modes,
    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Modes {
    /// Initialise a project/database to work with
    Init {
        #[command(subcommand)]
        input_method: DatasetChoice,
        #[command(flatten)]
        path: DatabasePath,
    },
    /// Add graphs to an already existing project  
    Add {
        #[command(subcommand)]
        input_method: DatasetChoice,
        #[command(flatten)]
        path: DatabasePath,
    },
    /// Compute invariants from a dataset
    Compute {
        #[command(flatten)]
        programs: ComputeChoice,
        #[clap(short, default_value = "3")]
        max_processes: usize,
        #[command(flatten)]
        path: DatabasePath,
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
        #[command(flatten)]
        output: OutputQueryArgs,
        /// The query to ask the database
        #[clap()]
        formula: String,
        #[command(flatten)]
        path: DatabasePath,
    },
    /// Show a summary of a project
    Summary {
        #[command(flatten)]
        path: DatabasePath,
    },
}

#[derive(Args, Debug, Clone)]
pub struct DatabasePath {
    /// The url to the database to connect to
    #[clap(default_value = DEFAULT_URL)]
    pub url: String,
}

#[derive(Args, Debug, Clone)]
pub struct GengArgs {
    /// The order(s) of the graphs to generate
    #[clap(name("order|min:[max] order"))]
    // TODO We should allow the user to exclude value from range
    pub order: String,
    /// The mininum and/or maximum number of edges of the graphs to generate
    #[clap(
        name("edges|min:[max] edges"),
        long("edges"),
        short('e'),
        value_delimiter = ':'
    )]
    pub edges: Vec<Option<String>>,

    /// The parameters to give to geng in order to generate a graph of an order
    #[clap(long, short)]
    pub params: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum DatasetChoice {
    /// Generates graph signatures using the `geng` command from the nauty package
    Geng {
        #[command(flatten)]
        args: GengArgs,
    },
    /// Imports graph signature from a file or the standard input
    Import {
        #[command(flatten)]
        args: ImportArgs,
    },
}

#[derive(Debug, clap::Args, Clone)]
#[group(required = false, multiple = false)] // This will stop the user from adding multiple arguments, meaning we do not have to check
pub struct ImportArgs {
    /// The file were the dataset to add is stored
    #[clap(short, long)]
    pub file: Option<String>,
    /// Read data from pipe (default)
    #[clap(short, long, action=ArgAction::SetTrue, default_value="true")]
    pub read_pipe: bool,
}

#[derive(Args, Debug, Clone)]
struct DatabaseInfoPath {
    /// The path or url to the database to access
    #[clap(short, long)]
    pub database_path_url: String,
}

#[derive(Debug, clap::Args, Clone)]
#[group(required = true, multiple = false)]
pub struct ComputeChoice {
    /// The configuration file to use for this workplace.
    #[clap(long, short)]
    pub config_file: Option<String>,
    /// The list of programs to call (each without any dependency).
    #[clap(long, short, value_parser, num_args = 1.., value_delimiter = ' ')]
    pub programs: Vec<String>,
}

#[derive(Debug, clap::Args, Clone)]
#[group(required = false, multiple = true)]
pub struct OutputQueryArgs {
    #[command(subcommand)]
    pub choice: Option<OutputChoice>,
    /// Saves the result to a file
    #[clap(short, long)]
    pub file: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum OutputChoice {
    /// Prints result line by line to the standart output
    Stream,
    /// Prints the result as a pretty table
    #[group(required = false, multiple = false)]
    Table {
        #[clap(long, action=ArgAction::SetTrue, default_value="false")]
        /// Stores the entire query results in the table
        full: bool,

        /// [n:m] Only stores the n first and the m last rows
        #[clap(long)]
        partial: Option<String>,
    },
}
