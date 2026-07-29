use indexmap::{IndexMap, IndexSet};
use is_executable::IsExecutable;
use log::debug;
use regex::Regex;
use std::{
    collections::HashMap,
    env,
    fmt::{Debug, Display},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdout, Command, Stdio},
};
use thiserror::Error;
use topo_sort::TopoSort;

use crate::{data_handler::data_types::ValueType, database_handler::ParsedFunction};

const INVARIANT_REGEX: &str = "^([a-z]|[A-Z]|_)(_|[a-z]|[A-Z]|[0-9])*$";

#[derive(Debug, Error)]
pub enum ModuleExecutionError {
    #[error("Failed to start executing the module \"{0}\"")]
    FailedExecution(String),
    #[error("Failed to write to the stdin of the module: \"{1}\", reason \"{0}\"")]
    FailedWriteStdin(io::Error, String),
    #[error("Failed to open the {0} of module \"{1}\"")]
    ClosedStream(String, String),
    #[error(
        "The module \"{0}\" finished it's execution earlier than expected : Exit status \"{1}\" | stderr : \n\"{2}\" "
    )]
    EarlyExit(String, String, String),
    #[error("The module \"{0}\" returned \"{1}\" values instead of \"{2}\"")]
    UnexpectedOutput(String, usize, usize),
    #[error("Tried to input \"{1}\" values instead of \"{2}\" to module \"{0}\"")]
    UnexpectedInput(String, usize, usize),
    #[error("The output function returned an error during the execution : \"{0}\"")]
    FailedOutput(String),
    #[error("The input function returned an error during the execution : \"{0}\"")]
    FailedInput(String),
}

// TODO: Encapsulate the errors into a wrapper that always contains important informations
// neccessary to identify the problematic module for the user (path, output, name, etc...)

#[derive(Debug, Error)]
pub enum ModuleError {
    #[error("Any module must require at least one parameter")]
    NoArgs,
    #[error("The given path \"{0}\" does not lead to a file")]
    InvalidPath(PathBuf),
    #[error("The given file at \"{0}\" is not executable")]
    NotExecutable(PathBuf),
    #[error("The given invariant name \"{0}\" is not part of any of the given modules")]
    UnknownInvariant(String),
    #[error("Encountered an IoError : \"{0}\"")]
    IoError(#[from] io::Error),
    #[error(
        "The given name \"{0}\" is not a valid invariant name. An invariant name must follow the following regex : ^([a-z]|[A-Z]|_)(_|[a-z]|[A-Z]|[0-9])*$"
    )]
    InvalidName(String),
    #[error(
        "The given invariant \"{0}\" is not present, yet it is a dependency of the module \"{1}\""
    )]
    MissingDependency(String, PathBuf),
    #[error("The module \"{0}\" depends on itself")]
    DependsOnSelf(String),
    #[error("The module returns a value with the same name as one needed as an argument: \"{0}\"")]
    ReturnSelf(String),
    #[error("Tried to add the following module \"{0}\" twice")]
    AlreadyAddedModule(PathBuf),
    #[error("Tried to add an invariant with the name \"{0}\" twice")]
    AlreadyAddedInvariant(String),
    #[error("Encountered a dependency cycle, thus making the invariant computation impossible")]
    DependencyCycle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionArg {
    pub name: String,
    pub data_type: ValueType,
}

impl FunctionArg {
    pub fn has_same_name(&self, other: &FunctionArg) -> bool {
        self.name == other.name
    }
}

impl From<(String, ValueType)> for FunctionArg {
    fn from(value: (String, ValueType)) -> Self {
        Self {
            name: value.0,
            data_type: value.1,
        }
    }
}

impl From<(&str, ValueType)> for FunctionArg {
    fn from(value: (&str, ValueType)) -> Self {
        (value.0.to_string(), value.1).into()
    }
}

/// Represent an executable file that can be used to compute graph invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    /// The *absolute* path to the module.
    pub exec_path: PathBuf,
    pub fn_name: String,
    pub args: Vec<FunctionArg>,
    /// The type of output to return
    pub output: ValueType,
    pub batch_size: Option<usize>,
}

/// Trait used to provide a stream of data to provided to a module
pub trait AsyncModuleInput<E> {
    /// Returns the next values to feed to the module. None if everything was already given.
    fn call(&mut self) -> impl std::future::Future<Output = Option<Result<Vec<String>, E>>> + Send;
}

/// Trait used to save the data returned from the module.
pub trait AsyncModuleOutput<E> {
    /// Provides the module's returned values.
    fn call(
        &mut self,
        values: Vec<String>,
    ) -> impl std::future::Future<Output = Result<(), E>> + Send;
}

impl Module {
    /// Creates a module using the given parameters
    pub fn new(
        exec_path: impl Into<String>,
        fn_name: impl Into<String>,
        args: Vec<impl Into<FunctionArg>>,
        output: ValueType,
        batch_size: Option<usize>,
    ) -> Result<Self, ModuleError> {
        if args.is_empty() {
            return Err(ModuleError::NoArgs);
        }
        let fn_name: String = fn_name.into();
        let args: Vec<FunctionArg> = args.into_iter().map(|n| n.into()).collect();

        let exec_path = Self::check_validity(exec_path.into(), &fn_name, &args)?;

        let i = Module {
            exec_path,
            fn_name,
            output,
            args,
            batch_size,
        };

        Ok(i)
    }

    /// Creates a module with only one argument, a graph.
    /// Useful when making modules that act as invariants.
    /// The argument will be called `graph` and be a [`ValueType::Graph`].
    pub fn new_invariant(
        exec_path: impl Into<String>,
        fn_name: impl Into<String>,
        output: ValueType,
        batch_size: Option<usize>,
    ) -> Result<Self, ModuleError> {
        let fn_name: String = fn_name.into();
        let args = vec![("graph", ValueType::Graph).into()];

        let exec_path = Self::check_validity(exec_path.into(), &fn_name, &args)?;

        let i = Module {
            exec_path,
            args,
            fn_name,
            output,
            batch_size,
        };

        Ok(i)
    }

    /// Checks if the given executable is valid and returns the correct path buf
    fn check_validity(
        exec_path: String,
        fn_name: &String,
        args: &[FunctionArg],
    ) -> Result<PathBuf, ModuleError> {
        let path = Self::get_path(exec_path)?;
        for val in args {
            Self::check_invariant_name_validity(val)?;
            if *fn_name == val.name {
                return Err(ModuleError::ReturnSelf(fn_name.clone()));
            }
        }
        Ok(path)
    }

    /// Checks if the given invariant path actually leads to the module
    fn get_path(exec_path: String) -> Result<PathBuf, ModuleError> {
        // Checks if the given file path exists
        let mut inv_path = Path::new(&exec_path);
        let curr_path = env::current_dir()?;
        let joined_path = curr_path.join(inv_path);

        // If the invariant path is not absolute,
        if !inv_path.is_absolute() {
            // Turn it into one
            inv_path = Path::new(&joined_path);
        }

        // and if the path doesn't lead to a file
        if let Ok(false) = inv_path.try_exists() {
            // Try to turn it into an absolute path
            return Err(ModuleError::InvalidPath(inv_path.to_path_buf()));
        }
        if !inv_path.is_executable() {
            return Err(ModuleError::NotExecutable(inv_path.to_path_buf()));
        }
        Ok(inv_path.to_path_buf())
    }

    /// Checks if the given invariant name can be used to create a table and/or a column in a database
    ///
    /// # Errors :
    /// * If the given name is not ascii
    /// * If the given name does not match with the following regex: [`INVARIANT_REGEX`]
    pub fn check_invariant_name_validity(value: &FunctionArg) -> Result<(), ModuleError> {
        let re = Regex::new(INVARIANT_REGEX).expect("Regex should be okay");
        if !value.name.is_ascii() || !re.is_match(&value.name) {
            return Err(ModuleError::InvalidName(value.name.to_string()));
        }
        Ok(())
    }
}
