use std::{fs::File, io, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::data_handler::invariant_execs::{InvariantError, InvariantsExecutable};

#[derive(Error, Debug)]
pub enum ConfigFileError {
    #[error("Could not open config file : `{0}`")]
    FileError(#[from] io::Error),
    #[error("Could not read json file : `{0}`")]
    JsonError(#[from] serde_json::Error),
    #[error("Could not create one of the given invariant : `{0}`")]
    InvariantError(#[from] InvariantError),
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
    /// The optional uri of the workplace
    uri: Option<String>,

    /// The number of data being sent between the executables and the databases
    batch_size: Option<usize>,

    /// The list of executables that should be executed by the program.
    executables: Vec<ExecutableJson>,
}

#[derive(Debug)]
/// Contains data that will affect how the program should be ran.
pub struct ConfigFile {
    /// The list of executables that should be executed by the program.
    pub executables: Vec<InvariantsExecutable>,
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
            for val in &mut config_json.executables {
                val.path = parent_path.join(&val.path).display().to_string();
            }
        }

        from_json_data(config_json)
    }

    pub fn read_json_value(value: Value) -> Result<Self, ConfigFileError> {
        from_json_data(serde_json::from_value(value)?)
    }
}

fn from_json_data(config_json: ConfigJsonFile) -> Result<ConfigFile, ConfigFileError> {
    let mut executables = vec![];

    for exec in config_json.executables {
        if let Some(dep) = exec.dep {
            executables.push(InvariantsExecutable::new(exec.path, exec.names, dep)?);
        } else {
            executables.push(InvariantsExecutable::new_no_dep(exec.path, exec.names)?);
        }
    }

    Ok(ConfigFile { executables })
}
