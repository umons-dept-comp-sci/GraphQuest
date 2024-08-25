
use clap::{ArgAction, Args, Parser, Subcommand};

use clap_verbosity_flag::{InfoLevel, WarnLevel};
use gquest_core::data_handler::invariant_handlers::{InvariantsExecutable, InvariantsOrderHandler};
use gquest_core::db_handler::{graph_database::*, sqlite_handler::*};
use gquest_core::data_handler::data_loaders::Method::*;
use gquest_core::utils::subject::Observer;
use log::{debug, info};
use std::ops::Range;
use std::fmt::Debug;
use std::fs::File;
use std::path::Path;


use crate::log_handler::*;
use crate::cli_commands::*;



/// Creates a workspace and adds a dataset using the prefered way of the user
pub async fn compute(path: DatabasePath, choice: ComputeChoice)
{
    println!("{:?}", choice);

    let mut inv_execs = InvariantsOrderHandler::new();

    // If user provided a list of programs, and he must at least provide one
    if choice.programs.len() != 0 {
        for pro_path in choice.programs {
            let inv_path = Path::new(&pro_path);
            let inv_name = String::from(inv_path.file_stem().expect(format!("Cannot get the non extension part of the given executable \"{pro_path}\"").as_str())
                                                .to_str().expect(format!("The given executable path \"{pro_path}\" cannot be turned into an invariant name").as_str()));
            let exec = InvariantsExecutable::new(&pro_path, vec![ inv_name],
                                                                     vec![], None, None, None);
            inv_execs.add_inv_exec(exec);
        }
    }
    // If user provided a dependency file
    else if let Some(dep_file) = choice.dependencies_file{
        
        inv_execs = InvariantsOrderHandler::read_json(&dep_file);
    }


}

// cargo run -- compute -d "/home/axel/Desktop/bir/GraphQuest/backend/resources/invariant_modules/dep.json"