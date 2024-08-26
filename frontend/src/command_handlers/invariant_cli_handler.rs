
use clap::{ArgAction, Args, Parser, Subcommand};

use clap_verbosity_flag::{InfoLevel, WarnLevel};
use gquest_core::data_handler::invariant_handlers::{InvariantsExecManager, InvariantsExecutable, InvariantsOrderHandler};
use gquest_core::db_handler::{graph_database::*, sqlite_handler::*};
use gquest_core::data_handler::data_loaders::Method::*;
use gquest_core::utils::subject::Observer;
use log::{debug, info};
use std::ops::Range;
use std::fmt::Debug;
use std::fs::File;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;


use crate::log_handler::*;
use crate::cli_commands::*;



/// Creates a workspace and adds a dataset using the prefered way of the user
pub async fn compute(path: DatabasePath, choice: ComputeChoice, max_processes: usize)
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
    // Apply topological order
    let execution_manager = inv_execs.get_topological_order();
    
    info!("Topological order of the execution: {}", execution_manager.pretty_string());
    
    // connect to database
    let mut wp: Workspace<SqliteGraphDatabase> = Workspace::connect_workspace(&path.url).await;
    
    // get length of the dataset
    let dataset_len = wp.get_dataset_length().await;

    // Get executable groups
    let groups = execution_manager.group_process_executions(max_processes);
    
    info!("Grouped executables, now starting the computation of invariants");
    // Init executables progress bar observer
    let mut progress_bars: Vec<ExecutableGroupObs> = vec![]; 
    for group in &groups {
        let mut t = ExecutableGroupObs::new();
        t.change_settings( dataset_len, group.get_group_file_names());
        
        progress_bars.push(t);
    }

    // Execute groups
    let mut i = 0;

    for group in groups {
        progress_bars[i].start();
        wp.execute_group(group, Some(&progress_bars[i])).await;
        info!("next");
        i += 1;
    }
    
    info!("Finished computing invariants, closing database");
    wp.close_workspace().await;

    



}

// cargo run -- compute -d "/home/axel/Desktop/bir/GraphQuest/backend/resources/invariant_modules/dep.json"