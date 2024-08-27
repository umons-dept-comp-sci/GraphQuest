use std::{cmp::min, collections::HashMap, fmt::{self, Debug, Display}, fs::File, io::{stdin, stdout, BufRead, Write}, path::Path, process::{id, Child, ChildStdin, ChildStdout, Command, Stdio}, sync::{mpsc, Arc, RwLock}, thread};
use std::io::BufReader;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use topo_sort::{SortResults, TopoSort};

use crate::db_handler::{graph_database::{GraphDatabase, DATASET_TABLE_NAME}, sqlite_handler::SqliteGraphDatabase};


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
    pub exec_path: String,

    /// The name of the computed invariants in the returned order
    pub names: Vec<String>,

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

impl Debug for InvariantsExecutable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InvExec").field("exec_path", &self.exec_path).finish()
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
        //println!("{:?}", p.parent());
        
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

    /// Returns the executable that produces the invariant with the given name
    fn get_executable(&self, name: &String) -> &InvariantsExecutable
    {
        &self.path_exec_hashmap[&self.name_path_hashmap[name]]
    }
 

    /// Returns all file names of the currently stored executables
    pub fn get_all_file_names(&self) -> Vec<String>
    {
        let mut res: Vec<String> = vec![];

        for p in self.path_exec_hashmap.keys() {
            let path = Path::new(p);
            res.push(String::from(path.file_name().unwrap().to_str().unwrap()));
        }
        res
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

    /// Groups all [InvariantsExecutable] by their dependencies and the number of simultaneous processes
    pub fn group_process_executions(mut self, mut proccess_available: usize) -> Vec<InvariantExecGroup>
    {
        let n = self.dep_index.len();
        if proccess_available < 1 {
            panic!("At least one proccess must be used to work with");
        }
        
        let mut res: Vec<InvariantExecGroup> = vec![];
        
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
            let task_copy = {
                let mut tmp: Vec<InvariantsExecutable> = vec![];
                for i in &can_exec {
                    tmp.push(self.executables[*i].clone());
                }
                tmp
            };
            let groups = InvariantExecGroup::group_by_dependencies(task_copy);
            
            //Self::exec_invariants(groups, db).await;
            res.push(groups);

            // update dependencies
            for j in &can_exec {    // for all executed programs
                proccess_available += 1;
                completed += 1;
                for dep_index in &self.dep_index[*j] {   // for all programs currently waiting for this program to finish
                    self.dep_left[*dep_index as usize] -= 1;   // update them
                }
            }
        }
        res

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


pub struct InvariantExecGroup
{
    group: Vec<Vec<InvariantsExecutable>>,
    min_values: Option<Vec<Vec<usize>>>,
    pub smallest_min: Option<usize>,
    pub dataset_len: Option<usize>,
    pub data_to_process: Option<usize>
}


impl InvariantExecGroup {

    fn new() -> Self
    {
        Self {
            group: vec![],
            min_values: None,
            dataset_len: None,
            smallest_min: None,
            data_to_process: None,
        }
    }

    /// Group all invariant executable using their dependencies 
    fn group_by_dependencies(mut inv_executables: Vec<InvariantsExecutable>) -> Self
    {
        let mut res = InvariantExecGroup::new();
        
        let mut i = 0;
        let mut groups = {
            let mut tmp: Vec<usize> = vec![];
            
            for _ in 0..inv_executables.len()
            {
                tmp.push(inv_executables.len());
            }

            tmp
        };
        let mut current_group = 0;

        while i < inv_executables.len() {
            // if not yet grouped
            if groups[i] == inv_executables.len() {
                // new group formed
                res.group.push(vec![]);
                groups[i] = current_group;
                let mut j = i + 1;
                while j < inv_executables.len() {
                    // if not yet grouped (if has been grouped we don't compare their dependency vectors)
                    // and if they have the same dependecy vectors, we can group them 
                    
                    if groups[j] == inv_executables.len() && &inv_executables[i].dependencies == &inv_executables[j].dependencies {
                        groups[j] = current_group;
                    }
                    j += 1;
                }
                current_group += 1; 
            }
            
            i += 1;
        }
        
        for i in (0..(inv_executables.len())).rev()
        {
            let exec = inv_executables.pop().unwrap();
            // push exec to correct group
            res.group[groups[i]].push(exec);
        }
        
        
        res
    }
   

    // Returns the len of the group
    pub fn len(&self) -> usize
    {
        let mut count = 0;
        for vec in &self.group{
            for _ in vec{
                count += 1;
            }
        }
        count
    }

    /// Fetches the current progression of the invariants by checking their table length (if they have one)
    pub async fn fetch_progress_info<'a, T: GraphDatabase<'a>>(&mut self, db: &T)
    {
        let max_size = db.get_size_of_table(DATASET_TABLE_NAME).await.unwrap();
        let mut smallest_min = max_size;
        let mut res:Vec<Vec<usize>> = vec![];
        let mut total_data = 0;
        for v in &self.group {
            let mut tmp_vec: Vec<usize> = vec![];
            for exec in v {
                let mut min = max_size;
                // We need to check the invariant that has the lowest number of values in its table
                for inv_name in &exec.names {
                    let size = {
                        match db.get_size_of_table(&InvariantsExecutable::get_table_name_from_string(inv_name)).await {
                            Ok(nb) => nb,
                            Err(_) => 0,    // Invariant was not computed before
                        }
                    };
                    if min > size {
                        min = size;
                    }   
                }
                total_data += min;
                tmp_vec.push(min);
                // Check if it is the smallest of the entire group
                if smallest_min > min {
                    smallest_min = min;
                }
                
            }
            res.push(tmp_vec);
        }
        self.min_values = Some(res);
        self.dataset_len = Some(max_size);
        self.smallest_min = Some(smallest_min);
        self.data_to_process = Some(total_data);

    }

    /// Compute the invariant and stores result in the database
    pub async fn exec_invariants<'a, T: GraphDatabase<'a>>(&mut self, db: &T)
    {
        // get min size
        if let None = self.smallest_min {
            self.fetch_progress_info(db).await;
        }
        
        
        
        fn exec_command(exec: &InvariantsExecutable) -> Child 
        { 
            Command::new(format!("{}", exec.exec_path))
                                    .stdin(Stdio::piped())
                                    .stdout(Stdio::piped())
                                    .spawn().unwrap()
        }

        
        let mut current_data = self.smallest_min.unwrap();

        while self.dataset_len.unwrap() > current_data
        {
            let mut stdout_vec: Vec<Vec<ChildStdout>> = vec![];
            
            // Push data
            let mut i = 0;
            for group in &self.group {
                let mut stdout_tmp: Vec<ChildStdout> = vec![];
                // We suppose that there is always at least one invariantsExecutable per group
                
                let mut stdin_vec: Vec<ChildStdin> = vec![];
                let mut j = 0;
                // Starts proccesses
                for exec in group {
                    if  self.min_values.clone().unwrap()[i][j] <= current_data {
                        let mut command = exec_command(exec); 
                        stdin_vec.push(command.stdin.take().unwrap());
                        stdout_tmp.push(command.stdout.take().unwrap());
                    }
                    j += 1;
                }
                stdout_vec.push(stdout_tmp);
                
                
                // Write data to the database, and close the stdins 
                db.fetch_data(Some(current_data), Some(BATCH_SIZE), &group[0].dependencies, stdin_vec).await.unwrap();
                i += 1;
            }

            // Read data
            for i in (0..stdout_vec.len()).rev() {
                let mut stdout_v = stdout_vec.pop().unwrap();
                for s in (0..stdout_v.len()).rev() {
                    let stdout = stdout_v.pop().unwrap();
                    let buf_read = BufReader::new(stdout);
                    let exec: &InvariantsExecutable = &self.group[i][s];
                    
                    db.push_data_from_buffer(exec, s, buf_read).await;
                }
            }
            
            
            current_data += BATCH_SIZE;
        }        
    }


    /// Returns all file names of the currently stored executables
    pub fn get_group_file_names(&self) -> Vec<String>
    {
       let mut res: Vec<String> = vec![];
       for vec in &self.group {
           for exec in vec{
               let path = Path::new(&exec.exec_path);
               res.push(String::from(path.file_name().unwrap().to_str().unwrap()));
           }
       }
       res
    }

}


impl Display for InvariantExecGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = String::new();
        let names = self.get_group_file_names();
        for name in names {
            res.push_str(&format!("{name}, "));
        }
        res.pop();res.pop();   // remove ", "
        write!(f, "({})", res)
    }
}