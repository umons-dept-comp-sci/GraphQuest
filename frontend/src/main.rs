use clap::{ArgAction, Args, Parser, Subcommand};

use clap_verbosity_flag::{InfoLevel, WarnLevel};
use gquest_core::db_handler::{graph_database::*, sqlite_handler::*};
use gquest_core::data_handler::data_loaders::Method::*;
use gquest_core::utils::subject::{Observer, TempGraphObs};
use log::{debug, info};
use std::ops::Range;
use std::fmt::Debug;


mod log_handler;

use log_handler::*;

const DEFAULT_URL: &str = "sqlite://gquest.db";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CliArg {
    #[command(subcommand)]
    cmd: Modes,
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity<WarnLevel>,

}


#[derive(Subcommand, Debug, Clone)]
enum Modes {
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
struct DatabasePath {
    /// The url to the database to connect to
    #[clap(default_value = DEFAULT_URL)]
    url: String  
}



#[derive(Args, Debug, Clone)]
struct GengArgs {
    /// The order(s) of the graphs to generate
    #[clap(name("order|min:[max] order"))]  // TODO We should allow the user to exclude value from range
    order : String,
    /// The mininum and/or maximum number of edges of the graphs to generate
    #[clap(name("edges|min:[max] edges"), long("edges"), short('e'), value_delimiter = ':')]
    edges : Vec<Option<String>>,
    
    /// The parameters to give to geng in order to generate a graph of an order
    #[clap(long, short)]
    params : Option<String>,
}
   



#[derive(Subcommand, Debug, Clone)]
enum DatasetChoice {
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
struct ImportArgs {
    /// The file were the dataset is stored
    #[clap(short, long)]
    file: Option<String>,
    /// Read from the standart input (default) 
    #[clap(short, long, action=ArgAction::SetTrue, default_value="true")]
    read_stdin : bool,
}



#[derive(Args, Debug, Clone)]
struct  DatabaseInfoPath {
    /// The path or url to the database to access
    #[clap(short,long)]
    database_path_url: String,
}


#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = CliArg::parse();
    startup_log(args.verbose);

    //test_progress();
    
    match args.cmd {
        Modes::Init { path, input_method } => init(path, input_method).await,
        Modes::Add {path, input_method } => add(path, input_method).await,
        Modes::Compute { path, dependencies_file, programs } => todo!(),
            
        Modes::Delete { path, table_name } => todo!(),
        Modes::Query { hide_output, do_not_save, formula, path } => todo!(),
        Modes::Summary { path } => todo!(),
    }
}


/// Creates a workspace and adds a dataset using the prefered way of the user
async fn init(path: DatabasePath, choice: DatasetChoice)
{
    info!("Creating workspace database at path : {:?}", path.url);
    // Create workspace
    

    // pb lifeline must end with workspace !
    let pb = DatasetProcess::create(8 as u64);

    let mut wp : Workspace<SqliteGraphDatabase> = Workspace::init_workspace(&path.url).await;

    info!("Database created");

    // Only one can be chosen at a time
    match_import_data(&mut wp, choice, &pb).await;
    // Close workspace
    info!("Closing database");
    wp.close_workspace().await;
}

/// Connects to a workspace and complete a dataset using the prefered way of the user 
async fn add(path: DatabasePath, choice: DatasetChoice)
{
    // Connect to workspace
    info!("Connecting to database at path : {:?}", path.url);
    let pb = DatasetProcess::create(20 as u64);
    let mut wp : Workspace<SqliteGraphDatabase> = Workspace::connect_workspace(&path.url).await;
    info!("Connected to database");
    match_import_data(&mut wp, choice, &pb).await;
    // close workspace
    wp.close_workspace().await;
}


/// Matches between the different dataset choices and calls the relevant function
async fn match_import_data<'a,  T: GraphDatabase<'a>> (wp: &mut Workspace<'a, T>, choice: DatasetChoice, progress_bar: &'a DatasetProcess)
{
    info!("Importing data");
    match choice {
        DatasetChoice::Geng { args } => geng_choice(wp, args, &progress_bar).await,
        DatasetChoice::Import { args } => import_choice(wp, args, &progress_bar).await,
    }
    info!("Finished importing data");
}


/// Imports a dataset from a call to geng which we construct here
async fn geng_choice<'a, T:  GraphDatabase<'a>> (wp: &mut Workspace<'a,T>, args: GengArgs, progress_bar: &'a DatasetProcess)
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
    // Init progress bar
    
    
    
    // params
    let mut params_arg = String::new();
    if let Some(p) = args.params {
        params_arg = format!("-{p}");
    }
    

    
    for order in iterator 
    {
        wp.add_dataset(GengAPI { nb_of_vertices: order, graph_settings: params_arg.clone(), edges_born: edges }, progress_bar ).await;
        
        
        progress_bar.notify(true);    // Update progress bar
    }
    
} 


/// Imports a dataset either from a file or from the stdin
async fn import_choice<'a,  T: GraphDatabase<'a>> (wp: &mut Workspace<'a, T>,  args: ImportArgs, progress_bar: &'a DatasetProcess)
{
    // A path was given
    if let Some(path) = args.file
    {
        //wp.add_dataset(File(path.clone())).await;
    }
    // if no file is given we suppose the input will arrive from stdin
    else {
        //wp.add_dataset(Stdin).await;
    }
}