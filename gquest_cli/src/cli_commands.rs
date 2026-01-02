use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};

const DEFAULT_URL: &str = "sqlite://gquest.db";

#[derive(Parser)]
#[command(
    author("Axel Foucart"),
    version,
    about("gquest: Developped by Axel Foucart at Algorithm Lab, UMONS-2024-2026")
)]
pub struct CliArg {
    #[command(subcommand)]
    pub cmd: Modes,
    #[command(flatten)]
    pub path: DatabasePath,
    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Modes {
    /// Add signatures to a database (and creates it if needed)
    #[command(subcommand_value_name = "SOURCE", subcommand_help_heading = "Sources")]
    Add {
        #[command(subcommand, name = "SOURCE")]
        input_method: DatasetChoice,
        #[command(flatten)]
        batch_size: BatchSizeArg,
    },
    /// Removes data from the database
    #[command(subcommand_value_name = "TARGET", subcommand_help_heading = "Targets")]
    Remove {
        #[command(subcommand, name = "TARGET")]
        choice: RemoveChoice,
    },
    /// Explore the dataset
    #[command(subcommand_value_name = "OUTPUT", subcommand_help_heading = "Outputs")]
    Query {
        #[command(subcommand, name = "OUTPUT")]
        output: Option<OutputChoice>,
        /// The path to the config file to use
        #[clap()]
        config_file: String,
        /// The query to ask the database
        #[clap()]
        query: String,
    },
    /// Show the tables present in the database
    Summary {
        /// [n:m] Only displays the n first and the m last rows. Can improve performances.
        #[clap(short)]
        partial: Option<String>,
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
    /// The order(s) of the graphs to generate.
    /// Either an order list (ex: "1,2,5") or range list (ex: "1:4, 6:10") with inclusive bounds.
    #[clap(name("(order | range) list"))]
    pub order: String,
    /// The addition parameters to give to geng
    pub params: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum DatasetChoice {
    /// Generates graph signatures using the `geng` command from the nauty package
    Geng {
        #[command(flatten)]
        args: GengArgs,
    },
    /// Imports graph signatures from a file
    File {
        /// The path to were the dataset to add is stored
        path: String,
    },
    /// Imports graph signatures from a pipe
    Pipe {},
}

// Used to not repeat the same field everywhere
#[derive(Args, Debug, Clone)]
struct DatabaseInfoPath {
    /// The path or url to the database to access
    #[clap(short, long)]
    pub database_path_url: String,
}

// Also used to not repeat the same field multiple times
#[derive(Args, Debug, Clone)]
pub struct BatchSizeArg {
    #[clap(short('b'), default_value("5000"))]
    pub batch_size: usize,
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

#[derive(Subcommand, Debug, Clone)]
pub enum OutputChoice {
    /// Stores the result as a `csv` file
    File {
        /// The path of the file
        path: String,
        /// The separator of the values
        #[clap(short, default_value = ",")]
        separator: char,
    },
    /// Prints result line by line to the standart output
    Stdout,
    /// Prints the result as a pretty table (default)
    #[group(required = false, multiple = false)]
    Table {
        /// [n:m] Only stores the n first and the m last rows. Can improve performances.
        #[clap(short)]
        partial: Option<String>,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum RemoveChoice {
    /// Removes all tables except the dataset.
    Invariants,
    /// Removes the dataset from the database.
    Dataset,
    /// Removes all tables from the database, even if not related to gquest !
    All,
}
