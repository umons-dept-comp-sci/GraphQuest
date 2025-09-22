use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use is_executable::IsExecutable;
use regex::Regex;
use thiserror::Error;
use topo_sort::TopoSort;

const INVARIANT_REGEX: &str = "^([a-z]|[A-Z]|_)(_|[a-z]|[A-Z]|[0-9])*$";

#[derive(Debug, Error)]
pub enum InvariantErrors {
    #[error("The given path \"{0}\" does not lead to a file")]
    InvalidPath(PathBuf),
    #[error("The given file at \"{0}\" is not executable")]
    NotExecutable(PathBuf),
    #[error("The given name \"{0}\" is not a valid invariant name. An invariant name must follow the following regex : ^([a-z]|[A-Z]|_)(_|[a-z]|[A-Z]|[0-9])*$")]
    InvalidName(String),
    #[error(
        "The given invariant \"{0}\" is not present, yet it is a dependency of the executable \"{1}\""
    )]
    MissingDependency(String, PathBuf),
    #[error("The executable \"{0}\" depends on itself")]
    DependsOnSelf(String),
    #[error("Tried to add the following executable \"{0}\" twice")]
    AlreadyAddedExecutable(PathBuf),
    #[error("Tried to add an invariant with the name \"{0}\" twice")]
    AlreadyAddedInvariant(String),
    #[error("Encountered a dependency cycle, thus making the invariant computation impossible")]
    DependencyCycle,
}

/// Represent an executable file that can be used to compute inveriants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantsExecutable {
    /// The *absolute* path to the file to execute.
    pub exec_path: PathBuf,

    /// The name of the computed invariants in the returned order
    pub invariant_names: Vec<String>,

    /// The vector of dependencies requiered to compute this invariant.
    /// A dependency cannot be present in the invariant names field.
    pub dependencies: Vec<String>,
}

impl InvariantsExecutable {
    /// Creates an invariant using the given parameters
    pub fn new(
        exec_path: String,
        names: Vec<String>,
        dependencies: Vec<String>,
    ) -> Result<Self, InvariantErrors> {
        let exec_path = Self::check_validity(exec_path, &names, &dependencies)?;

        let i = InvariantsExecutable {
            exec_path,
            invariant_names: names,
            dependencies,
        };

        Ok(i)
    }

    /// Checks if the given executable is valid and returns the correct path buf
    fn check_validity(
        exec_path: String,
        names: &[String],
        dependencies: &[String],
    ) -> Result<PathBuf, InvariantErrors> {
        let path = Self::get_path(exec_path)?;
        for name in names {
            Self::check_invariant_name_validity(name)?;
            if dependencies.contains(name) {
                return Err(InvariantErrors::DependsOnSelf(name.to_string()));
            }
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
            return Err(InvariantErrors::InvalidPath(inv_path.to_path_buf()));
        } else if !inv_path.is_executable() {
            return Err(InvariantErrors::NotExecutable(inv_path.to_path_buf()));
        }
        Ok(inv_path.to_path_buf())
    }

    /// Checks if the given invariant name can be used to create a table and/or a column in a database
    ///
    /// # Errors :
    /// * If the given name is not ascii
    /// * If the given name does not match with the following regex: [`INVARIANT_REGEX`]
    pub fn check_invariant_name_validity(name: &String) -> Result<(), InvariantErrors> {
        let re = Regex::new(INVARIANT_REGEX).expect("Regex should be okay");
        if !name.is_ascii() || !re.is_match(name) {
            return Err(InvariantErrors::InvalidName(name.to_string()));
        }
        Ok(())
    }
}

/// Stores [`InvariantsExecutable`]s in order to prepare a topology sort.
/// Such as by checking :
/// * if an invariant was already added
/// * if one of it's dependencies does not exists
#[derive(Default)]
pub struct ExecutableSorter {
    /// `Invariant name` -> `Linked Executable path`
    name_path_hashmap: HashMap<String, PathBuf>,
    /// `Executable path` -> `Executables`
    path_exec_hashmap: HashMap<PathBuf, InvariantsExecutable>,
}

impl ExecutableSorter {
    /// Creates a new empty [`ExecutableSorter`]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_inv_exec(&mut self, inv: InvariantsExecutable) -> Result<(), InvariantErrors> {
        if self.path_exec_hashmap.contains_key(&inv.exec_path) {
            return Err(InvariantErrors::AlreadyAddedExecutable(
                inv.exec_path.clone(),
            ));
        }
        for name in &inv.invariant_names {
            if self.name_path_hashmap.contains_key(name) {
                return Err(InvariantErrors::AlreadyAddedInvariant(name.to_string()));
            }
            self.name_path_hashmap
                .insert(name.to_string(), inv.exec_path.clone());
        }
        self.path_exec_hashmap.insert(inv.exec_path.clone(), inv);
        Ok(())
    }

    /// Performs a topological sort with the stored [`InvariantsExecutable`]s
    /// # Errors
    /// * If one of the dependencies from one invariant is not present.
    /// * If a cycle is found.
    pub fn sort(mut self) -> Result<Vec<InvariantsExecutable>, InvariantErrors> {
        // Init the topological sort
        let mut topo_sort: TopoSort<PathBuf> =
            TopoSort::with_capacity(self.path_exec_hashmap.len());

        // Add nodes
        for (path, exec) in &self.path_exec_hashmap {
            // Check that all dependencies of this executable are present
            // And gather all executable path that this executable is relying on
            let mut dependencies: Vec<PathBuf> = vec![];
            for dep in &exec.dependencies {
                match self.name_path_hashmap.get(dep) {
                    Some(path) => {
                        dependencies.push(path.clone());
                    }
                    None => {
                        return Err(InvariantErrors::MissingDependency(
                            dep.to_string(),
                            exec.exec_path.clone(),
                        ));
                    }
                }
            }
            // Add this exec as a node with dependencies to the graph
            topo_sort.insert(path.clone(), dependencies);
        }
        // Apply topological sort
        let ordered_paths = match topo_sort.into_vec_nodes() {
            topo_sort::SortResults::Full(items) => items,
            topo_sort::SortResults::Partial(_) => {
                return Err(InvariantErrors::DependencyCycle);
            }
        };

        let mut res = vec![];
        for path in ordered_paths {
            res.push(
                self.path_exec_hashmap
                    .remove(&path)
                    .expect("present in hashmap"),
            );
        }

        Ok(res)
    }
}
