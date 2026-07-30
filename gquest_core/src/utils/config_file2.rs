use std::{
    collections::{HashMap, hash_map::Entry},
    fs::File,
    io,
    path::Path,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::data_handler::{
    data_types::ValueTypeError,
    module::{TypedArg, Module, ModuleError},
};

#[derive(Error, Debug)]
pub enum ConfigFileError {
    #[error("Could not open config file at \"{1}\" because : `{0}`")]
    FileError(io::Error, String),
    #[error("Could not read json file : `{0}`")]
    JsonError(#[from] serde_json::Error),
    #[error("Could not create one of the given invariant : `{0}`")]
    InvariantError(#[from] ModuleError),
    #[error("At least one invariant should be provided in the file")]
    NoInvariantError,
    #[error("Could not create a module : `{0}`")]
    ArgParseError(#[from] ValueTypeError),
    #[error("The modules at the paths \"{0}\" and \"{1}\" have the same function name.")]
    SameFunctionNameError(String, String),
}

/// Private struct simply used to not directly create a module.
#[derive(Serialize, Deserialize)]
struct ModuleJson {
    /// The path to the file to execute.
    pub path: String,
    /// The name of the function
    pub function: String,
    pub args: Option<Vec<ArgJson>>,
    pub output: String,
    pub batch_size: Option<usize>,
}

#[derive(Serialize, Deserialize)]
struct ArgJson {
    pub name: String,
    pub class: String,
}

/// Private struct simply used to not directly create a config file.
#[derive(Serialize, Deserialize)]
struct ConfigJsonFile {
    /// The precision to use when checking if two numbers are equal or not. By default set to 0.
    epsilon: Option<f64>,
    /// The number of data being sent between the executables and the databases
    batch_size: Option<usize>,
    /// The maximum number of threads to use to use when computing invariants
    nb_threads: Option<usize>,
    /// The list of executables that should be executed by the program.
    modules: Vec<ModuleJson>,
    /// A list of aliases that will be replaced with the given value.
    pub aliases: Option<Vec<(String, String)>>,
}

#[derive(Debug)]
/// Contains the necessary data to initialise a workplace.
/// Important to mention that at least one invariant will be present and checked for any instantiation errors.
pub struct ConfigFile {
    // /// The precision to use when checking if two numbers are equal or not. By default set to 0.
    // epsilon: Option<f64>,
    /// The maximum number of data being sent between the modules and the databases.
    batch_size: usize,
    /// The maximum number of threads to use when computing invariants
    nb_threads: usize,
    /// An hashmap containing the modules that the program can use : `Function name` -> `Module`.
    modules: HashMap<String, Module>,
    // /// The list of aliases : `key` -> `value`
    // aliases: Vec<(String, String)>,
}

impl ConfigFile {
    /// Converts a json file at the given location to a [`ConfigFile`].
    /// # Errors
    /// * [`ConfigFileError::FileError`] : if something went wrong when trying to open the given file.
    /// * [`ConfigFileError::JsonError`] : if the file content could not be turned into a [`ConfigFile`].
    ///     * In this case check if the fields name and content are aligned with the struct fields.
    pub fn read_json_file(path: &String) -> Result<Self, ConfigFileError> {
        let p = Path::new(&path);
        let f = match File::open(p) {
            Ok(f) => f,
            Err(e) => {
                return Err(ConfigFileError::FileError(e, path.to_string()));
            }
        };
        let mut config_json: ConfigJsonFile = serde_json::from_reader(f)?;

        if let Some(parent_path) = p.parent() {
            for val in &mut config_json.modules {
                val.path = parent_path.join(&val.path).display().to_string();
            }
        }

        from_json_data(config_json)
    }

    /// Converts a json value to a [`ConfigFile`].
    /// # Errors
    /// * [`ConfigFileError::FileError`] : if something went wrong when trying to open the given file.
    /// * [`ConfigFileError::JsonError`] : if the file content could not be turned into a [`ConfigFile`].
    ///     * In this case check if the fields name and content are aligned with the struct fields.
    pub fn read_json_value(value: Value) -> Result<Self, ConfigFileError> {
        from_json_data(serde_json::from_value(value)?)
    }

    pub fn get_module_refs(&self) -> &HashMap<String, Module> {
        &self.modules
    }

    pub fn get_batch_size(&self) -> usize {
        self.batch_size
    }
    pub fn get_nb_threads(&self) -> usize {
        self.nb_threads
    }
    // pub fn get_epsilon(&self) -> &Option<f64> {
    //     &self.epsilon
    // }
    // pub fn get_aliases(&self) -> &Vec<(String, String)> {
    //     &self.aliases
    // }
}

fn from_json_data(config_json: ConfigJsonFile) -> Result<ConfigFile, ConfigFileError> {
    if config_json.modules.is_empty() {
        return Err(ConfigFileError::NoInvariantError);
    }
    let mut modules: HashMap<String, Module> = HashMap::new();

    for module in config_json.modules {
        match modules.entry(module.function.clone()) {
            Entry::Occupied(occupied_entry) => {
                return Err(ConfigFileError::SameFunctionNameError(
                    occupied_entry
                        .get()
                        .exec_path
                        .as_os_str()
                        .to_str()
                        .expect("correct path string") // it was inputted as a string so this is safe
                        .to_string(),
                    module.path,
                ));
            }
            Entry::Vacant(vacant_entry) => {
                let module = if let Some(args_json) = module.args {
                    // From JSON to real module types
                    let mut args = Vec::with_capacity(args_json.len());
                    for arg in args_json {
                        args.push(TypedArg {
                            name: arg.name,
                            data_type: arg.class.try_into()?,
                        });
                    }
                    Module::new(
                        module.path,
                        module.function,
                        args,
                        module.output.try_into()?,
                        module.batch_size,
                    )
                } else {
                    Module::new_invariant(
                        module.path,
                        module.function,
                        module.output.try_into()?,
                        module.batch_size,
                    )
                }?;
                vacant_entry.insert_entry(module);
            }
        }
    }

    Ok(ConfigFile {
        // epsilon: config_json.epsilon,
        modules,
        nb_threads: config_json.nb_threads.unwrap_or(1),
        batch_size: config_json.batch_size.unwrap_or(5000),
        // aliases: config_json.aliases.unwrap_or_default(),
    })
}
