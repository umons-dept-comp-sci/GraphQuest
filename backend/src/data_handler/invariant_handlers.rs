use std::{cmp::min, collections::HashMap, fmt::{self, Display}, fs::File, io::{stdin, stdout, BufRead, Write}, path::Path, process::{id, Command, Stdio}, sync::{mpsc, Arc, RwLock}, thread};
use std::io::BufReader;
use serde::{Deserialize, Serialize};
use topo_sort::{SortResults, TopoSort};

use crate::db_handler::{graph_database::GraphDatabase, sqlite_handler::SqliteGraphDatabase};


/// The prefix of all the invariant tables 
pub const INVARIANT_PREFIX : &str = "inv_";
/// The quantity of data to send to the invariant executable
pub const BATCH_SIZE: usize = 3000;


/// Private struct simply used to help the json parsing of multiple executables 
#[derive(Serialize, Deserialize)]
struct _InvariantVec {
    executables : Vec<InvariantsExecutable>
}

/// A struct used to represent invariants to be computed/ or used for compution
#[derive(Serialize, Deserialize)]
pub struct InvariantsExecutable {
    /// The path to the file to execute.
    /// 
    /// Can be either a relative or absolute path
    exec_path: String,

    /// The name of the computed invariants in the returned order
    names: Vec<String>,

    /// The vector of dependencies requiered to compute this invariant.
    /// A dependency can be either :
    /// * The name of a required invariant 
    /// * The name of the executable of an invariant
    dependencies: Vec<String>,

    ///// When specified, the program will be provided the needed input that matches the given query
    //input_query: Option<String>

    /// The return type of the program, written in the standart output. Integers by default
    return_type: Option<String>,

    /// The character that separates two inputs being read by the executable of this invariant
    /// 
    /// By default, will be `\n`
    input_seperator: Option<char>,

    /// The character that separates two computed results of the executable of this invariant being read by this program
    /// 
    /// By default, will be `\n`
    output_separator: Option<char>

}

impl Display for InvariantsExecutable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Customize so only `x` and `y` are denoted.
        write!(f, "{:?}", self.names)
    }
}


impl Clone for InvariantsExecutable {
    fn clone(&self) -> Self {
        Self { exec_path: self.exec_path.clone(), names: self.names.clone(), dependencies: self.dependencies.clone(), return_type: self.return_type.clone(), input_seperator: self.input_seperator.clone(), output_separator: self.output_separator.clone() }
    }
}




impl InvariantsExecutable {
    /// Creates an invariant using the given name
    pub fn new(exec_path: &String, names: Vec<String>, dependencies: Vec<String>, return_type: Option<String>, 
               input_seperator: Option<char>, output_seperator: Option<char>) -> Self
    {
        
        let path = Path::new(exec_path);
        path.try_exists().expect(format!("The given invariant path \"{exec_path}\", is not valid").as_str());

        for name in &names {
            if !name.is_ascii() {
                panic!("The given invariant name \"{name}\" is not valid")
            }
        }

        let i = InvariantsExecutable {
            exec_path : exec_path.to_string(),
            names,
            dependencies,
            return_type,
            input_seperator,
            output_separator: output_seperator
        };
        i.check_path();

        i

    }

    /// Checks if the given invariant path actually leads to the executable
    /// ## Exceptions
    /// Will panic if the path doesn't lead to an executable 
    fn check_path(&self)
    {
        // Checks if the given file path exists
        let tmp_clone = self.exec_path.clone();
        let inv_path = Path::new(&tmp_clone);
        
        // if the invariant doesn't exist
        if let Ok(false) = inv_path.try_exists() {
            
            panic!("The given file path (\"{}\"),\n\t to the invariants (\"{:?}\") is not valid", self.exec_path, self.names);
        }
    }


    // Simply formats the name to what the invariant table name is
    pub fn get_table_name_from_string(name: &String) -> String 
    {
        INVARIANT_PREFIX.to_string() + name
    }

    /// Compute the invariant and stores result in the database
    pub async fn exec_inv<'a, T: GraphDatabase<'a>>(&self, db: &T)
    {
        // get min size

        let mut nb_of_data = 
        {
            let mut table_vec:Vec<String> = vec![];
            for inv_name in &self.dependencies {
                table_vec.push(Self::get_table_name_from_string(inv_name));
            }
            db.get_min_dependency_size(&table_vec).await
        };

        let mut batch_sent = 0;
        while nb_of_data > 0 {
            let mut ex_inv = Command::new(format!("{}", self.exec_path))
                                    .stdin(Stdio::piped())
                                    .stdout(Stdio::piped())
                                    .spawn().unwrap();
            
            let stdin = ex_inv.stdin.take().unwrap();

            db.fetch_data(Some(batch_sent * BATCH_SIZE), Some(BATCH_SIZE), vec![], &stdin).await.unwrap();

            drop(stdin); // forces stdin of program to stop reading so we can read the stdout of it
            
            {
                let stdout = ex_inv.stdout.as_mut().unwrap();
                let stdout_reader = BufReader::new(stdout);
                
                db.push_data_from_buffer(self, stdout_reader).await;
                
                drop(ex_inv);
            }
            batch_sent += 1;
            nb_of_data -=  min(BATCH_SIZE, nb_of_data); // min used to avoid substraction overflow
        }
        
        println!("finished fetching");
        
        
        //FIXME Here is the problem: We are sending so much data that the python program reads, but he also sends them back to the output pipe
        // but since we are currently sending the data, we cannot read it
        // so this means that the output pipe gets filled up and the python program waits to write more
        // but since he is waiting, and can't read what we are sending
        // therefore the stdin pipe also gets full
        // i.e. deadlock  

                  
        //println!("FOO: {}", String::from_utf8_lossy(&ex_inv.stdout));
    
        
    }
    // Copy of function
    /*
    pub async fn exec_inv<'a, T: GraphDatabase<'a>>(&self, db: &T)
    {
        // executes the given invariant
        //let to_pipe = Command::new(format!("rev"))
        //            .stdout(Stdio::piped())
        //            .spawn()
        //            .unwrap();
        //
        //let mut out_to_in = to_pipe.stdin.unwrap();


        
        
        db.fetch_dataset_signatures(None, out_to_in).await;
        let mut ex_inv = Command::new(format!("{}", self.exec_path))
                            .stdin(Stdio::from(to_pipe.stdout.unwrap()))
                            .spawn().unwrap();
         

        {
            let stdout = ex_inv.stdout.as_mut().unwrap();
            let stdout_reader = BufReader::new(stdout);
            for line in stdout_reader.lines() {
                if let Ok(sign) = line {
                    println!("Just read : {:?}", sign);
                }
            }
        }

        ex_inv.wait().unwrap();
                  
        //println!("FOO: {}", String::from_utf8_lossy(&ex_inv.stdout));
    
        
    }
     */
}


/// Stores [InvariantsExecutable]s in order to prepare the later computations involving them.
/// Such as by checking :
/// * if an invariant was already added
/// * if one of it's dependencies was not added 
/// * ...
/// And can perform a topology sort
pub struct InvariantsOrderHandler
{
    name_path_hashmap : HashMap<String, String>,
    path_exec_hashmap : HashMap<String, InvariantsExecutable>,
    nb_of_exec: usize,
}

impl InvariantsOrderHandler {
    /// Creates a new empty [InvariantsOrderHandler]
    pub fn new() -> Self 
    {
        Self {
            name_path_hashmap: HashMap::new(),
            path_exec_hashmap: HashMap::new(),
            nb_of_exec: 0,
        }
    }
    
    /// Creates a [InvariantsHandler] from a given json file
    /// 
    /// The given paths in the dependency files can be either 
    /// * Absolute
    /// * Or relative to the dependency file itself 
    pub fn read_json(path: &String) -> Self
    {
        let mut handler = Self::new();
        let p = Path::new(&path);
        let f = File::open(p).expect(format!("The given dependency file path (\"{path}\") is not valid").as_str());
        println!("{:?}", p.parent());
        
        let inv_vec: _InvariantVec = serde_json::from_reader(f).expect(format!("The given dependency file (\"{path}\") format is not correct").as_str());
        let mut inv_path: &Path;
        let mut tmp_clone: String;
        for mut inv in inv_vec.executables {

            tmp_clone = inv.exec_path.clone();
            inv_path = Path::new(&tmp_clone);
            // If the path given is not absolute
            // it means that the executable is related to the position of the given dependency file
            if !inv_path.is_absolute() {
                // If the given dependency file has a parent dir path, we can add it
                if let Some(s) = p.parent() {
                    inv.exec_path = format!("{}/{}", s.to_str().unwrap(), &inv.exec_path);
                }
            }
            // Checks if the given file path exists
            inv.check_path();
            
            handler.add_inv_exec(inv);
        }

        handler
    }

    fn get_executable(&self, name: &String) -> &InvariantsExecutable
    {
        &self.path_exec_hashmap[&self.name_path_hashmap[name]]
    }

    

    /// Adds an [InvariantsExecutable] to the [InvariantsHandler] 
    pub fn add_inv_exec(&mut self, inv: InvariantsExecutable)
    {
        for inv_name in &inv.names {
            
            if self.name_path_hashmap.contains_key(inv_name) {
                panic!("The executable \"{}\" has an invariant with the name \"{inv_name}\" that already exists in the executable \"{}\"", inv.exec_path, self.get_executable(inv_name).exec_path)
            }
            self.name_path_hashmap.insert(inv_name.clone(), inv.exec_path.clone());
        }
        self.path_exec_hashmap.insert(inv.exec_path.clone(), inv);
        self.nb_of_exec += 1;
    }
    
    /// Performs a topological sort with the stored [InvariantsExecutable]s
    /// 
    /// After this function, the [InvariantsOrderHandler] will go out of scope.
    pub fn get_topological_order(mut self) -> InvariantsExecManager
    {
        // Init the topological sort
        let mut topo_sort: TopoSort<String> = TopoSort::with_capacity(self.nb_of_exec);
        let mut prev_exec_path : String = String::new();
        // Add nodes
        let mut dependencies: Vec<String>;
        let mut exec : &InvariantsExecutable;
        for (inv_name, inv_path) in &self.name_path_hashmap {
            dependencies = vec![];
            exec = &self.path_exec_hashmap[inv_path];
            for dep_name in &exec.dependencies {
                // Check that the dependency exists
                if !self.name_path_hashmap.contains_key(dep_name.as_str()) {
                    panic!("The following dependency \"{}\" from the exectuable \"{}\", has not been added to this InvariantsOrderHandler", dep_name, exec.exec_path);
                }
                if dep_name == inv_name
                {
                    panic!("The executable \"{}\" cannot depend on itself",  exec.exec_path);
                }
                dependencies.push(self.get_executable(dep_name).exec_path.clone());
            }
            
            if prev_exec_path != exec.exec_path {
                prev_exec_path = exec.exec_path.clone();
                topo_sort.insert(exec.exec_path.clone(), dependencies);
            }
        }
        // Apply topological sort
        let result_string = match topo_sort.into_vec_nodes() {
            SortResults::Full(nodes) => nodes,
            SortResults::Partial(_) => panic!("A dependency cycle was found for the given invariants, thus making their computations impossible !"),
        };
        
        //let mut result_nodes: Vec<&TopologicalInvariantNode> = vec![];
        let mut res = InvariantsExecManager::new();
        let mut name_index_hash: HashMap<String, u16> = HashMap::new();
        
        
        let mut exec: InvariantsExecutable;
        let mut index = 0;
        let mut dep_left: u16;
        for inv_path in result_string {
            // Move the ownership of the invariant from the hashmap (by removing it) to the result vector
            exec = self.path_exec_hashmap.remove(&inv_path).unwrap();

            for name in &exec.names {
                name_index_hash.insert( name.clone(),index);    // insert into hashmap for an easy access to his index
            }
        
            
            // Add dependencies
            for dep_name in &exec.dependencies {
                // By definition of a topological sort,
                // the dependencies were already added to the hashmap
                res.add_dep( index, *name_index_hash.get(dep_name).unwrap());
            }

            dep_left = exec.dependencies.len() as u16;
            res.add_nodes(exec, dep_left);
            index += 1;
        }
        res
    }
}





/// Manages the execution of invariants whose order has been determined by an [InvariantsOrderHandler]
pub struct InvariantsExecManager
{
    /// The invariants sorted using a topological sort
    executables: Vec<InvariantsExecutable>,
    /// The number of dependence left to execute for each invariants
    dep_left: Vec<u16>,
    /// The index of the invariants relying on a specific invariant
    dep_index: Vec<Vec<u16>>,
}


impl InvariantsExecManager {
    /// Creates a new [InvariantExecManager]
    fn new() -> Self
    {
        Self {
            executables: vec![],
            dep_left : vec![],
            dep_index : vec![], 
        }
    }

    /// Adds a node to execute after the ones already added
    fn add_nodes(&mut self, inv: InvariantsExecutable, dep_left: u16)
    {
        self.executables.push(inv);
        self.dep_left.push(dep_left);
        self.dep_index.push(vec![]);
    }

    /// Adds a dependence to the already placed invariant 
    fn add_dep(&mut self, inv_index: u16, dep_index: u16)
    {
        self.dep_index[dep_index as usize].push(inv_index);
    }

    /// 
    pub async fn handle_process_executions(mut self, mut proccess_available: usize)
    {
        let n = self.dep_index.len();
        if proccess_available < 1 {
            panic!("At least one proccess must be used to work with");
        }

        // Vector used in order to keep track of what executable are left to execute
        let mut todo : Vec<bool> = {
            let mut tmp: Vec<bool> = vec![];
            for _ in 0..n {
                tmp.push(true);
            }
            tmp
        };
        let mut can_exec : Vec<usize>;
        let mut completed = 0;
        let mut i;

        while completed < n {
            can_exec = vec![];
            i = 0;
            // Check which tasks can now be executed
            while i < n && proccess_available > 0
            {
                if self.dep_left[i] == 0  && todo[i]{
                    can_exec.push(i);
                    proccess_available -= 1;
                    todo[i] = false;
                }
                i += 1;
            }
            // execute those tasks
            println!("executing : {:?}", can_exec);
            // update dependencies
            for j in &can_exec {    // for all executed programs
                proccess_available += 1;
                completed += 1;
                for dep_index in &self.dep_index[*j] {   // for all programs currently waiting for this program to finish
                    self.dep_left[*dep_index as usize] -= 1;   // update them
                }
            }
        }

    }
   
    /// Start a thread that will execute the invariant located at the given index
    async fn start_inv(&self, inv_index: usize, original_sender: &mpsc::Sender<u16>, db_url: String)
    {
        let tx_copy = mpsc::Sender::clone(original_sender);
        let inv_copy = self.executables[inv_index].clone();
        
        
        
        //let db = Arc::new(RwLock::new(T::connect_graph_database(&db_url.clone()).await));
        
        //let tmp = SqliteGraphDatabase::connect_graph_database(&db_url).await;
        // Create (if it wasn't already added) the invariant in the database
        //tmp.init_invariant(&inv_copy).await;
        
        tx_copy.send(inv_index as u16).unwrap();

        //tmp.close_connection().await;
        
    }


    /// Gets a formatted string to help show the given topological sort
    pub fn pretty_string(&self) -> String
    {
        let mut to_print = String::new();
        for (i, inv) in self.executables.iter().enumerate() {
            to_print += inv.to_string().as_str();
            if i != self.executables.len()-1 {
                to_print.push_str(" => ");
            }
        }
        to_print
    }
}
