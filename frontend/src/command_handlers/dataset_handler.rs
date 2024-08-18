
use clap::{ArgAction, Args, Parser, Subcommand};

use clap_verbosity_flag::{InfoLevel, WarnLevel};
use gquest_core::db_handler::{graph_database::*, sqlite_handler::*};
use gquest_core::data_handler::data_loaders::Method::*;
use gquest_core::utils::subject::Observer;
use log::{debug, info};
use std::ops::Range;
use std::fmt::Debug;
use std::fs::File;


use crate::log_handler::*;
use crate::cli_commands::*;

/// Creates a workspace and adds a dataset using the prefered way of the user
pub async fn init(path: DatabasePath, choice: DatasetChoice)
{
    info!("Creating workspace database at path : {:?}", path.url);
    // Create workspace
    
    // pb lifeline must end with workspace's !
    let mut pb = DatasetPbObs::create();
    
    let mut wp : Workspace<SqliteGraphDatabase> = Workspace::init_workspace(&path.url).await;

    info!("Database created");

    // Only one can be chosen at a time
    match_import_data(&mut wp, choice, &mut pb).await;
    // Close workspace
    info!("Closing database");
    wp.close_workspace().await;
}

/// Connects to a workspace and complete a dataset using the prefered way of the user 
pub async fn add(path: DatabasePath, choice: DatasetChoice)
{
    // Connect to workspace
    info!("Connecting to database at path : {:?}", path.url);
    let mut pb = DatasetPbObs::create();
    let mut wp : Workspace<SqliteGraphDatabase> = Workspace::connect_workspace(&path.url).await;
    info!("Connected to database");
    match_import_data(&mut wp, choice, &mut pb).await;
    // close workspace
    info!("Closing database");
    wp.close_workspace().await;
}


/// Matches between the different dataset choices and calls the relevant function
async fn match_import_data<'a,  T: GraphDatabase<'a>> (wp: &mut Workspace<'a, T>, choice: DatasetChoice, progress_bar: &'a mut DatasetPbObs)
{
    info!("Importing data");
    match choice {
        DatasetChoice::Geng { args } => geng_choice(wp, args, progress_bar).await,
        DatasetChoice::Import { args } => import_choice(wp, args, progress_bar).await,
    }
    info!("Finished importing data");
}


/// Imports a dataset from a call to geng which we construct here
async fn geng_choice<'a, T:  GraphDatabase<'a>> (wp: &mut Workspace<'a,T>, args: GengArgs, progress_bar: &'a mut DatasetPbObs)
{
    debug!("Given geng args: {:?}", args);
    let mut edges: (Option<u32>, Option<u32>) = (None, None);
    
    fn string_to_u32_option(opt_str: &Option<String>) -> Option<u32> 
    {
        if let Some(str) = opt_str{
            if str != ""{
                Some(str.parse::<u32>().expect(format!("The given value ({str}) is not a positive number").as_str()))
            }
            else {
                None
            }
        }
        else {
            None
        }
    } 
    
    // Edges
    if args.edges.len() == 1 {
        // only one edge
        // If the user gave an edge
        let r = string_to_u32_option(&args.edges[0]);
        edges = (r, r);
    }
    // Gave a born
    if args.edges.len() == 2 {
        let min_born = string_to_u32_option(&args.edges[0]);
        let max_born = string_to_u32_option(&args.edges[1]);
        edges = (min_born, max_born);
    }
    
    // read order
    // If ':' is present it means we were supplied with a born
    let mut iterator: Range<u32> = 0..0;
    if args.order.contains(':') {
        
        let order_born: Vec<&str> = args.order.split(":").collect();
        
        if order_born.len() == 1 {
            
        }
        // Create the iterator
        else if order_born.len() == 2{
            let min_border = string_to_u32_option(&Some(order_born[0].to_string())); 
            let max_border = string_to_u32_option(&Some(order_born[1].to_string())); 
            match (min_border, max_border) {
                (None, None) => panic!("You must at least provide a maximum value for the order"),
                (None, Some(n)) => iterator = 0..(n+1), // 
                (Some(_), None) => panic!("You must also provide a maximum value for the order"),
                (Some(n), Some(m)) => {
                    if n > m
                    {
                        panic!("The given minimum value({n}) must be smaller than the maximum value({m}) for the order");
                    } 
                    iterator = n..(m+1);
                },
            }
        }
        if order_born.len() > 2 {
            panic!("Too many arguments for the \"order\" option: {:?}", order_born);
        }
        
    }// Just one order
    else {
        let order = string_to_u32_option(&Some(args.order.to_string())).unwrap(); 
        iterator = order..(order + 1); // + 1 so as to at least execute once for the given order
    }
    // params
    let mut params_arg = String::new();
    if let Some(p) = args.params {
        params_arg = format!("-{p}");
    }
    

    progress_bar.start_progress(iterator.len() as u64, format!("Order"), ProgressBarType::Iterating);
    for order in iterator 
    {
        wp.add_dataset(GengAPI { nb_of_vertices: order, graph_settings: params_arg.clone(), edges_bound: edges }, progress_bar ).await;
        progress_bar.notify_iteration();    // Update progress bar
    }
    
} 


/// Imports a dataset either from a file or from the stdin
async fn import_choice<'a,  T: GraphDatabase<'a>> (wp: &mut Workspace<'a, T>,  args: ImportArgs, progress_bar: &'a mut DatasetPbObs)
{
    // A path was given
    if let Some(path) = args.file
    {
        {
            let f = File::open(&path).expect(format!("The given file path \"{path}\" is not valid").as_str());
            progress_bar.start_progress(f.metadata().unwrap().len(), String::from("bytes"), ProgressBarType::Download);
        }
        
        wp.add_dataset(File(path.clone()), progress_bar).await;
    }
    // if no file is given we suppose the input will arrive from stdin
    else {
        progress_bar.start_progress(10, String::from("reading"), ProgressBarType::Reading);
        wp.add_dataset(Stdin, progress_bar).await;
        // Since we don't know the actual end of the pb, we need to manually finish it
        progress_bar.force_finish();
    }
}