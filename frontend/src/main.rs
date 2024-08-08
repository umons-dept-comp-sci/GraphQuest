use std::{option, path::PathBuf};

use clap::{ArgAction, Args, Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CLI_ARG {
    #[command(subcommand)]
    cmd: Modes
}

#[derive(Subcommand, Debug, Clone)]
enum Modes {
    /// Mode to handle the databases and various tables that will
    ///  be storing the results of the various computations made by this program.
    #[command(subcommand)]
    Database(DatabaseModes),
    /// Mode to compute invariants/properties of graph using datasets.
    Invariant {
        #[command(flatten)]
        database_information : DatabaseInfoPath,
        /// The path to the file containing the dependencies required by the given list of programs
        #[clap(default_value = "None")]
        dependencies_file: std::path::PathBuf,
        /// The list of programs to call
        #[clap(long, short, value_parser, num_args = 1.., value_delimiter = ' ')]
        programs: Vec<std::path::PathBuf>,
    },
    /// Mode to send querries to the various created datasets.
    Querry {
        #[command(flatten)]
        database_information : DatabaseInfoPath,
        #[clap(long, short, action=ArgAction::SetTrue)]
        is_true: bool,
    }
}

#[derive(Subcommand, Debug, Clone)]
enum DatabaseModes {
    /// Adds a table to the given database
    Add,
    /// Removes a table from the given database
    Remove,
    /// Shows a table from the given database
    Show
}




#[derive(Args,  Debug, Clone)]
struct  DatabaseInfoPath {
    /// The path or url to the database to access
    database_path_url: String,
    /// The name of the table to use
    table_name: String
}

fn main() {
     
    let args = CLI_ARG::parse();
    
    match args.cmd {
        Modes::Querry{ database_information: _, is_true } => println!("set ??? : {is_true}"),
        Modes::Database(_) => todo!(),
        Modes::Invariant { database_information, dependencies_file, programs } => 
        {
            println!("
                {:?},
                {:?},
                {:?}
            ", database_information, dependencies_file, programs)
        },
    }
}
