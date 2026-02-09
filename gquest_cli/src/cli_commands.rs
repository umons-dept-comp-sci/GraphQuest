use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};

const DEFAULT_URL: &str = "sqlite://gquest.db";
const DEFAULT_CONFIGS: &str = "configs.json";

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
    /// Adds signatures to a database (and creates it if needed)
    #[command(
        alias = "a",
        subcommand_value_name = "SOURCE",
        subcommand_help_heading = "Sources"
    )]
    Add {
        #[command(subcommand, name = "SOURCE")]
        input_method: DatasetChoice,
        #[command(flatten)]
        batch_size: BatchSizeArg,
    },
    /// Removes data from the database
    #[command(
        alias = "r",
        subcommand_value_name = "TARGET",
        subcommand_help_heading = "Targets"
    )]
    Remove {
        #[command(subcommand, name = "TARGET")]
        choice: RemoveChoice,
    },
    /// Explores the dataset.
    #[command(
        alias = "q",
        subcommand_value_name = "OUTPUT",
        subcommand_help_heading = "Outputs"
    )]
    Query {
        #[command(flatten)]
        args: QueryArgs,
    },
    /// Tries to find counter examples in the dataset.
    #[command(
        alias = "c",
        subcommand_value_name = "OUTPUT",
        subcommand_help_heading = "Outputs"
    )]
    Counter {
        #[command(flatten)]
        args: QueryArgs,
    },
    /// Shows the tables present in the database
    #[command(alias = "s")]
    Summary {
        /// [n:m] Only displays the n first and the m last rows. Can improve performances and visibility.
        #[clap(short)]
        partial: Option<String>,
    },
}

#[derive(Args, Debug, Clone)]
pub struct QueryArgs {
    /// The query to ask the database
    #[clap()]
    pub query: String,
    #[command(subcommand, name = "OUTPUT")]
    pub output: Option<OutputChoice>,
    /// The path to the config file to use
    #[clap(default_value = DEFAULT_CONFIGS)]
    pub config_file: String,
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
    /// The command to use to call the geng program. Can sometimes be `nauty-geng` instead.
    #[clap(default_value = "geng")]
    pub command_name: String,
    /// The addition parameters to give to geng
    pub params: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum DatasetChoice {
    #[clap(alias = "g")]
    /// Generates graph signatures using the `geng` command from the nauty package
    Geng {
        #[command(flatten)]
        args: GengArgs,
    },
    #[clap(alias = "f")]
    /// Imports graph signatures from a file
    File {
        /// The path to were the dataset to add is stored
        path: String,
    },
    #[clap(alias = "p")]
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
        /// [n:m] Only stores the n first and the m last rows. Can improve performances and visibility.
        #[clap(short)]
        partial: Option<String>,
    },
}

/// Removes data from the database
#[derive(Subcommand, Debug, Clone)]
pub enum RemoveChoice {
    // #[command(alias = "i")]
    // /// Removes an invariant from the databe.
    // Invariant {
    //     /// The name of the invariant to remove.
    //     name: String,
    // },
    #[command(alias = "ai")]
    /// Removes all tables except the dataset.
    AllInvariant,
    #[command(alias = "d")]
    /// Removes the dataset from the database.
    Dataset,
    #[command(alias = "a")]
    /// Removes all tables from the database, even if not related to gquest !
    All,
}
