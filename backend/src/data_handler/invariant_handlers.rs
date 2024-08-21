use std::{collections::HashMap, fmt::{self, Display}, fs::File, path::Path, process::{id, Command, Stdio}, sync::mpsc, thread};
use serde::{Deserialize, Serialize};
use topo_sort::{SortResults, TopoSort};


/// The prefix of all the invariant tables 
pub const INVARIANT_PREFIX : &str = "inv_";



/// Private struct simply used to help the json parsing of multiple invariants 
#[derive(Serialize, Deserialize)]
struct _InvariantVec {
    invariants : Vec<Invariant>
}

/// A struct used to represent invariants to be computed/ or used for compution
#[derive(Serialize, Deserialize)]
pub struct Invariant {
    /// The path to the file to execute.
    /// 
    /// Can be either a relative or absolute path
    exec_path: String,

    /// The name of the computed invariant
    name: String,

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

impl Display for Invariant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Customize so only `x` and `y` are denoted.
        write!(f, "{}", self.name)
    }
}


impl Clone for Invariant {
    fn clone(&self) -> Self {
        Self { exec_path: self.exec_path.clone(), name: self.name.clone(), dependencies: self.dependencies.clone(), return_type: self.return_type.clone(), input_seperator: self.input_seperator.clone(), output_separator: self.output_separator.clone() }
    }
}




impl Invariant {
    /// Creates an invariant using the given name
    pub fn new(exec_path: &String, name: &String, dependencies: Vec<String>, return_type: Option<String>, 
               input_seperator: Option<char>, output_seperator: Option<char>) -> Self
    {
        
        let path = Path::new(exec_path);
        path.try_exists().expect(format!("The given invariant path \"{exec_path}\", is not valid").as_str());

        if !name.is_ascii() {
            panic!("The given invariant name \"{name}\" is not valid")
        }

        let i = Invariant {
            exec_path : exec_path.to_string(),
            name : name.to_string(),
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
            
            panic!("The given file path (\"{}\"),\n\t to the invariant (\"{}\") is not valid", self.exec_path, self.name);
        }
    }



    // Simply formats the invariant name to be easier to work with
    pub fn get_table_name(&self) -> String
    {
        // FIXME ATTENTION USER INPUT ET TABLE NAMES,
        INVARIANT_PREFIX.to_string() + &self.name    // Append the prefix to the invariant 
    }

    /// Compute the invariant by using the provided executable 
    pub fn exec_inv(&self)
    {
        // executes the given invariant
        let to_pipe = Command::new(format!("geng"))
                .arg("1")
                .arg("-q")
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
    
        let ex_inv = Command::new(format!("{}", self.exec_path))
                          .stdin(Stdio::from(to_pipe.stdout.unwrap()))
                          .output().unwrap();
        //println!("FOO: {}", String::from_utf8_lossy(&ex_inv.stdout));
    
        
    }
}


/// Stores invariants in order to prepare the later computations involving them.
/// Such as by checking :
/// * if an invariant was already added
/// * if one of it's dependencies was not added 
///
/// And can perform a topology sort
pub struct InvariantsOrderHandler
{
    name_hashmap : HashMap<String, Invariant>
}

impl InvariantsOrderHandler {
    /// Creates a new empty [InvariantsHandler]
    pub fn new() -> Self 
    {
        Self {
            name_hashmap : HashMap::new()
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
        for mut inv in inv_vec.invariants {

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
            
            handler.add_invariant(inv);
        }

        handler
    }

    

    /// Adds an invariant to the [InvariantsHandler] 
    pub fn add_invariant(&mut self, inv: Invariant)
    {
        let key = inv.name.clone();
        if self.name_hashmap.contains_key(&key) {
            panic!("An invariant with the same name \"{key}\" already exists")
        }
        self.name_hashmap.insert(inv.name.clone(), inv);
    }
    
    /// Performs a topological sort with the stored [Invariant]s
    /// 
    /// After this function, the [InvariantsHandler] will go out of scope.
    pub fn get_topological_order(mut self) -> InvariantExecManager
    {
        // Init the topological sort
        let mut topo_sort: TopoSort<String> = TopoSort::with_capacity(self.name_hashmap.len());
        // Add nodes
        let mut dependencies: Vec<String>;
        for (inv_name, inv) in &self.name_hashmap {
            dependencies = vec![];
            for dep_name in &inv.dependencies {
                // Check that the dependency exists
                if !self.name_hashmap.contains_key(dep_name.as_str()) {
                    panic!("The following dependency \"{}\" from the invariant \"{}\", has not been added to this invariantsHandler", dep_name, inv_name);
                }
                if dep_name == inv_name
                {
                    panic!("The invariant \"{inv_name}\" cannot depend on itself");
                }
                dependencies.push(dep_name.clone());
            }
            topo_sort.insert(inv_name.clone(), dependencies);
        }
        
        // Apply topological sort
        let result_string = match topo_sort.into_vec_nodes() {
            SortResults::Full(nodes) => nodes,
            SortResults::Partial(_) => panic!("A dependency cycle was found for the given invariants, thus making their computations impossible !"),
        };

        //let mut result_nodes: Vec<&TopologicalInvariantNode> = vec![];
        let mut res = InvariantExecManager::new();
        let mut name_index_hash: HashMap<String, u16> = HashMap::new();
        
        
        let mut inv: Invariant;
        let mut index = 0;
        let mut dep_left: u16;
        for inv_name in result_string { 
            name_index_hash.insert( inv_name.clone(),index);    // insert into hashmap for an easy access to his index
            // Move the ownership of the invariant from the hashmap (by removing it) to the result vector
            inv = self.name_hashmap.remove(&inv_name).unwrap();      
            
            // Add dependencies
            for dep_name in &inv.dependencies {
                // By definition of a topological sort,
                // the dependencies were already added to the hashmap
                res.add_dep( index, *name_index_hash.get(dep_name).unwrap());
                
            }
            dep_left = inv.dependencies.len() as u16;
            res.add_nodes(inv, dep_left);
            index += 1;
        }
        res
    }
}





/// Manages the execution of invariants whose order has been determined by an [InvariantsOrderHandler]
pub struct InvariantExecManager
{
    /// The invariants sorted using a topological sort
    invariants: Vec<Invariant>,
    /// The number of dependence left to execute for each invariants
    dep_left: Vec<u16>,
    /// The index of the invariants relying on a specific invariant
    dep_index: Vec<Vec<u16>>,
}


impl InvariantExecManager {
    /// Creates a new [InvariantExecManager]
    fn new() -> Self
    {
        Self {
            invariants: vec![],
            dep_left : vec![],
            dep_index : vec![], 
        }
    }

    /// Adds a node to execute after the ones already added
    fn add_nodes(&mut self, inv: Invariant, dep_left: u16)
    {
        self.invariants.push(inv);
        self.dep_left.push(dep_left);
        self.dep_index.push(vec![]);
    }

    /// Adds a dependence to the already placed invariant 
    fn add_dep(&mut self, inv_index: u16, dep_index: u16)
    {
        self.dep_index[dep_index as usize].push(inv_index);
    }

    /// Handles the execution in a topological order of the invariants by using the provided number of threads 
    pub fn handle_execution(mut self, mut threads_available: usize)
    {
        let n = self.dep_index.len();


        //println!("{}", self.pretty_print_order());
        if threads_available <= 1 {
            panic!("At least one thread must be used to work with");
        }

        // Vector used in order to keep track of what invariants are left to execute
        let mut todo : Vec<bool> = {
            let mut tmp: Vec<bool> = vec![];
            for _ in 0..n {
                tmp.push(true);
            }
            tmp
        };

        //println!("Starting thread manager");
        //println!("{:?}", self.dep_left);
        // Use of a "Multi-producer, single-consumer" struct in order to communicate with threads
        let (tx, rx) = mpsc::channel::<u16>();
        
        
        // Min since you can have more threads than process
        for i in 0..(std::cmp::min(threads_available, self.invariants.len())) {
            
            if self.dep_left[i] == 0 {
                self.start_thread(i, &tx);
                threads_available -= 1;
                todo[i] = false;    // i is now being worked on 
            }
            // It can sometimes happen that you can start invariants even if they are after 
            // ones waiting for a dependence, hence the full loop
        }
        //println!("Finished to init threads");
        
        // Counter of finished tasks
        let mut finish_count = 0;
        // When a message is received, it means the thread finished and it sent the index of the finished task
        for received in &rx {
            finish_count += 1;
            //println!("Was finished: {}", received);
            threads_available += 1;
            
            // Because this task is done, we can update the dependencies
            for d in &self.dep_index[received as usize] {
                self.dep_left[*d as usize] -= 1;
            }
            // Check if some other tasks are now available
            let mut i = 0;
            while threads_available > 0 && i < n {
                if todo[i] && self.dep_left[i] == 0
                {
                    threads_available -= 1;
                    todo[i] = false;
                    self.start_thread(i, &tx);                    
                }
                i += 1;
            }
            // Check if all tasks are finished or not
            if finish_count == self.invariants.len() 
            {
                break;
            }
            //println!("state: {:?}", todo);
        }
    }

    /// Start a thread that will execute the invariant located at the given index
    fn start_thread(&self, inv_index: usize, original_sender: &mpsc::Sender<u16>)
    {
        let tx_copy = mpsc::Sender::clone(original_sender);
        
        let inv_copy = self.invariants[inv_index].clone();
        println!("Gonna do: {}", inv_copy.name); 
        thread::spawn(move || 
        {
            // Execute invariant
            inv_copy.exec_inv();
            // send notification to main thread to signal the end of this program's execution
            tx_copy.send(inv_index as u16).unwrap();
        });
    }


    /// Gets a formatted string to help show the given topological sort
    pub fn pretty_print_order(&self) -> String
    {
        let mut to_print = String::new();
        for (i, inv) in self.invariants.iter().enumerate() {
            to_print += inv.to_string().as_str();
            if i != self.invariants.len()-1 {
                to_print.push_str(" => ");
            }
        }
        to_print
    }
}