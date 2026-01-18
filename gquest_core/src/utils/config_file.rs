use std::{fs::File, io, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::data_handler::invariant_execs::{Module, ModuleError};

#[derive(Error, Debug)]
pub enum ConfigFileError {
    #[error("Could not open config file : `{0}`")]
    FileError(#[from] io::Error),
    #[error("Could not read json file : `{0}`")]
    JsonError(#[from] serde_json::Error),
    #[error("Could not create one of the given invariant : `{0}`")]
    InvariantError(#[from] ModuleError),
    #[error("At least one invariant should be provided in the file")]
    NoInvariantError,
}

/// Private struct simply used to not directly create an executable.
#[derive(Serialize, Deserialize)]
struct ExecutableJson {
    /// The path to the file to execute.
    pub path: String,

    /// The name of the computed invariants in the returned order
    pub names: Vec<String>,

    /// The vector of dependencies requiered to compute this invariant.
    /// A dependency cannot be present in the invariant names field.
    pub dep: Option<Vec<String>>,
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
    modules: Vec<ExecutableJson>,
}

#[derive(Debug)]
/// Contains the necessary data to initialise a workplace.
/// Important to mention that at least one invariant will be present and checked for any instantiation errors.
pub struct ConfigFile {
    /// The precision to use when checking if two numbers are equal or not. By default set to 0.
    epsilon: Option<f64>,
    /// The number of data being sent between the executables and the databases
    batch_size: usize,
    /// The maximum number of threads to use when computing invariants
    nb_threads: usize,
    /// The list of executables that should be executed by the program.
    modules: Vec<Module>,
}

impl ConfigFile {
    /// Converts a json file at the given location to a [`ConfigFile`].
    /// # Errors
    /// * [`ConfigFileError::FileError`] : if something went wrong when trying to open the given file.
    /// * [`ConfigFileError::JsonError`] : if the file content could not be turned into a [`ConfigFile`].
    ///     * In this case check if the fields name and content are aligned with the struct fields.
    pub fn read_json_file(path: &String) -> Result<Self, ConfigFileError> {
        let p = Path::new(&path);
        let f = File::open(p)?;
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

    pub fn get_execs_ref(&self) -> &Vec<Module> {
        &self.modules
    }

    pub fn get_batch_size(&self) -> usize {
        self.batch_size
    }
    pub fn get_nb_threads(&self) -> usize {
        self.nb_threads
    }
    pub fn get_epsilon(&self) -> &Option<f64> {
        &self.epsilon
    }
}

fn from_json_data(config_json: ConfigJsonFile) -> Result<ConfigFile, ConfigFileError> {
    if config_json.modules.is_empty() {
        return Err(ConfigFileError::NoInvariantError);
    }
    let mut executables = vec![];

    for exec in config_json.modules {
        if let Some(dep) = exec.dep {
            executables.push(Module::new(exec.path, exec.names, dep)?);
        } else {
            executables.push(Module::new_no_dep(exec.path, exec.names)?);
        }
    }

    Ok(ConfigFile {
        epsilon: config_json.epsilon,
        modules: executables,
        nb_threads: config_json.nb_threads.unwrap_or(1),
        batch_size: config_json.batch_size.unwrap_or(1000),
    })
}
