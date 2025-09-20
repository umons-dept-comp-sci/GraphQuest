use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

use regex::Regex;
use thiserror::Error;

const INVARIANT_REGEX: &str = "^([a-z]|[A-Z]|_)(_|[a-z]|[A-Z]|[0-9])*$";

#[derive(Debug, Error)]
pub enum InvariantErrors {
    #[error("The given path \"{0}\" does not lead to a valid execution path")]
    InvalidPath(String),
    #[error("The given name \"{0}\" is not a valid invariant name. An invariant name must follow the following regex : ^([a-z]|[A-Z]|_)(_|[a-z]|[A-Z]|[0-9])*$")]
    InvalidName(String),
    #[error("The given dependency \"{0}\" is missing")]
    MissingDependency(String),
    #[error("Encountered a dependency cycle, thus making the invariant computation impossible")]
    DependencyCycle,
}

/// Represent an executable file that can be used to compute inveriants.
#[derive(Debug, Clone)]
pub struct InvariantsExecutable {
    /// The *absolute* path to the file to execute.
    pub exec_path: PathBuf,

    /// The name of the computed invariants in the returned order
    pub names: Vec<String>,

    /// The vector of dependencies requiered to compute this invariant.
    /// A dependency must be the name of a required invariant
    pub dependencies: Vec<String>,
}

impl InvariantsExecutable {
    /// Creates an invariant using the given parameters
    pub fn new(
        exec_path: String,
        names: Vec<String>,
        dependencies: Vec<String>,
    ) -> Result<Self, InvariantErrors> {
        let exec_path = Self::check_validity(exec_path, &names)?;

        let i = InvariantsExecutable {
            exec_path,
            names,
            dependencies,
        };

        Ok(i)
    }

    /// Checks if the given executable is valid and returns the correct path buf
    fn check_validity(exec_path: String, names: &Vec<String>) -> Result<PathBuf, InvariantErrors> {
        let path = Self::get_path(exec_path)?;
        for name in names {
            Self::check_invariant_name_validity(name)?;
        }
        Ok(path)
    }

    /// Checks if the given invariant path actually leads to the executable
    fn get_path(exec_path: String) -> Result<PathBuf, InvariantErrors> {
        // Checks if the given file path exists
        let tmp_clone = exec_path.clone();
        let inv_path = Path::new(&tmp_clone);

        // if the invariant doesn't exist
        if let Ok(false) = inv_path.try_exists() {
            return Err(InvariantErrors::InvalidPath(tmp_clone));
        }
        Ok(inv_path.to_path_buf())
    }

    /// Checks if the given invariant name can be used to create a table and/or a column in a database
    ///
    /// # Errors :
    /// * If the given name is not ascii
    /// * If the given name does not match with the following regex: `^([a-z]|[A-Z]|_)(_|[a-z]|[A-Z]|[0-9])*$`
    pub fn check_invariant_name_validity(name: &String) -> Result<(), InvariantErrors> {
        let re = Regex::new(INVARIANT_REGEX).expect("Regex should be okay");
        if !name.is_ascii() || !re.is_match(name) {
            return Err(InvariantErrors::InvalidName(name.to_string()));
        }
        Ok(())
    }
}

// fn exec_command(exec_path: String) -> Child {
//     Command::new(format!("{}", exec_path))
//         .stdin(Stdio::piped())
//         .stdout(Stdio::piped())
//         .spawn()
//         .expect("Could not execute command")
// }
