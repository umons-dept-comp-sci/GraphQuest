use std::{collections::HashMap, fmt::{self, Display}, fs::File, path::Path, process::{Command, Stdio}};
use serde::{Deserialize, Serialize};
use serde_json::Result;
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
}


/// Stores invariants in order to prepare the later computations involving them.
/// Such as by checking :
/// * if an invariant was already added
/// * if one of it's dependencies was not added 
///
/// And can perform a topology sort
pub struct InvariantsHandler
{
    name_hashmap : HashMap<String, Invariant>
}

impl InvariantsHandler {
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
    pub fn get_topological_order(mut self) -> Vec<Invariant>
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
        // Get result as invariant structs
        let mut res: Vec<Invariant> = vec![];
        for inv_name in result_string {
            // Move the ownership of the invariant from the hashmap (by removing it) to the result vector
            res.push(self.name_hashmap.remove(&inv_name).unwrap());
        }
        res
    }

    /// Gets a formatted string to help show the given topological sort
    pub fn pretty_print_order(res: &Vec<Invariant>) -> String
    {
        let mut to_print = String::new();
        for (i, inv) in res.iter().enumerate() {
            to_print += inv.to_string().as_str();
            if i != res.len()-1 {
                to_print.push_str(" => ");
            }
        }
        to_print
    }


}




pub fn exec_inv(inv: &Invariant)
{
    // executes the given invariant
    let to_pipe = Command::new(format!("geng"))
            .arg("5")
            .arg("-q")
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

    let ex_inv = Command::new(format!("{}", inv.exec_path))
                      .stdin(Stdio::from(to_pipe.stdout.unwrap()))
                      .output().unwrap();
    println!("FOO: {}", String::from_utf8_lossy(&ex_inv.stdout));

    
}


