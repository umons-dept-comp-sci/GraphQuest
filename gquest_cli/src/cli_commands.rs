use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};

cfg_if::cfg_if! {
    if #[cfg(feature = "sqlite")] {
        const DEFAULT_URL: &str = "sqlite://gquest.db";
    } else if #[cfg(feature = "postgres")] {
        const DEFAULT_URL: &str = "postgresql://localhost/gquest";
    } else {
        const DEFAULT_URL: &str = "sqlite://gquest.db"; // Fallback default
    }
}

const DEFAULT_CONFIGS: &str = "configs.json";

#[derive(Parser)]
#[command(
    author("Axel Foucart"),
    version,
    about("gquest: Developped by Axel Foucart at the Algorithms Lab, UMONS-2024-2026")
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
    /// Returns the result of a raw sql query sent to the database.
    /// The syntax of the queries here are dependent on the database system used.
    #[command(
        alias = "s",
        subcommand_value_name = "OUTPUT",
        subcommand_help_heading = "Outputs"
    )]
    Sql {
        #[command(flatten)]
        args: QueryArgs,
    },
    /// Sends a query to the database and returns the results.
    #[command(
        alias = "q",
        subcommand_value_name = "OUTPUT",
        subcommand_help_heading = "Outputs"
    )]
    Query {
        #[command(flatten)]
        contents: QueryContents,
    },
    // /// Shows the tables present in the database
    // #[command(alias = "sm")]
    // Summary {
    //     #[command(subcommand, name = "OUTPUT")]
    //     output: Option<Output>,
    // },
}

#[derive(Args, Debug, Clone)]
pub struct QueryContents {
    #[command(flatten)]
    pub args: QueryArgs,
    /// Additional expressions to append to the query result
    #[clap(default_value = None, short, long)]
    pub add_expr: Option<String>,
    #[command(flatten)]
    pub config: ConfigFileArg,
}

#[derive(Args, Debug, Clone)]
pub struct QueryArgs {
    /// The query to ask the database
    #[clap()]
    pub query: String,
    #[clap(flatten)]
    pub output: OutputSettings,
    /// Negates the last condition from the given query
    #[clap(short, long, default_value("false"))]
    pub counter: bool,
    /// Only retains the signature column and additional expressions.
    #[clap(short, long, default_value("false"))]
    pub retain_sigs: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ConfigFileArg {
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
        /// The path of the file to read
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

// #[derive(Subcommand, Debug, Clone)]
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum TableOutputFormat {
    Table,
    PlainText,
    Latex,
    Markdown,
}

#[derive(Args, Debug, Clone)]
pub struct OutputSettings {
    /// (n:m) Only displays the n first and the m last rows.
    #[clap(short)]
    pub partial: Option<String>,
    /// Hides the index column of the table.
    #[clap(short, long, default_value("false"))]
    pub no_id: bool,
    /// The format of the result.
    #[arg(short, long, value_enum, default_value_t = TableOutputFormat::Table)]
    pub format: TableOutputFormat,

    /// Saves the entire output as a csv file
    #[clap(flatten)]
    pub output_file: OutputFile,
}

#[derive(Args, Debug, Clone)]
pub struct OutputFile {
    /// Stores the entire result as a `csv` file at the given path
    #[arg(short, long)]
    pub output_path: Option<String>,
    /// The separator of the values
    #[clap(short, default_value = ",", requires = "output_path")]
    pub separator: char,
}

/// Removes data from the database
#[derive(Subcommand, Debug, Clone)]
pub enum RemoveChoice {
    #[command(alias = "i")]
    /// Removes an invariant from the databe.
    Invariant {
        /// The name of the invariant to remove.
        name: String,
    },
    #[command(alias = "ai")]
    /// Removes all tables except the dataset.
    AllInvariant,
    #[command(alias = "d")]
    /// Removes the dataset from the database.
    Dataset,
    #[command(alias = "a")]
    /// Removes all tables from the database (even those unrelated to gquest)
    All,
}
