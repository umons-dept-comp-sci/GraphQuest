use is_executable::IsExecutable;
use log::{debug, error};
use regex::Regex;
use std::{
    collections::HashMap,
    env,
    fmt::Display,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdout, Command, Stdio},
};
use thiserror::Error;
use topo_sort::TopoSort;

const MAX_STDIN_SIZE: usize = 40;
const INVARIANT_REGEX: &str = "^([a-z]|[A-Z]|_)(_|[a-z]|[A-Z]|[0-9])*$";

#[derive(Debug, Error)]
pub enum InvariantExecutionError {
    #[error("Failed to start executing the executable \"{0}\"")]
    FailedExecution(String),
    #[error("Failed to write to the stdin of the executable \"{1}\", reason \"{0}\"")]
    FailedWriteStdin(io::Error, String),
    #[error("The executable \"{0}\" finished it's execution earlier than expected : Exit status \"{1}\" | stderr : \n\"{2}\" ")]
    EarlyExit(String, String, String),
}

#[derive(Debug, Error)]
pub enum InvariantError {
    #[error("The given path \"{0}\" does not lead to a file")]
    InvalidPath(PathBuf),
    #[error("The given file at \"{0}\" is not executable")]
    NotExecutable(PathBuf),
    #[error("Encountered an IoError : \"{0}\"")]
    IoError(#[from] io::Error),
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
    pub dependencies: Option<Vec<String>>,
}

impl InvariantsExecutable {
    /// Creates an invariant using the given parameters
    pub fn new(
        exec_path: impl Into<String>,
        names: Vec<impl Into<String>>,
        dependencies: Vec<impl Into<String>>,
    ) -> Result<Self, InvariantError> {
        let invariant_names: Vec<String> = names.into_iter().map(|n| n.into()).collect();

        let dependencies: Option<Vec<String>> = {
            if dependencies.is_empty() {
                None
            } else {
                Some(dependencies.into_iter().map(|n| n.into()).collect())
            }
        };

        let exec_path =
            Self::check_validity(exec_path.into(), &invariant_names, dependencies.as_deref())?;

        let i = InvariantsExecutable {
            exec_path,
            invariant_names,
            dependencies,
        };

        Ok(i)
    }

    /// Creates an invariant with no dependency using the given parameters.
    pub fn new_no_dep(
        exec_path: impl Into<String>,
        names: Vec<impl Into<String>>,
    ) -> Result<Self, InvariantError> {
        let invariant_names: Vec<String> = names.into_iter().map(|n| n.into()).collect();

        let exec_path = Self::check_validity(exec_path.into(), &invariant_names, None)?;

        let i = InvariantsExecutable {
            exec_path,
            invariant_names,
            dependencies: None,
        };

        Ok(i)
    }

    /// Checks if the given executable is valid and returns the correct path buf
    fn check_validity(
        exec_path: String,
        names: &[String],
        dependencies: Option<&[String]>,
    ) -> Result<PathBuf, InvariantError> {
        let path = Self::get_path(exec_path)?;
        for name in names {
            Self::check_invariant_name_validity(name)?;
            if let Some(dependencies) = dependencies {
                if dependencies.contains(name) {
                    return Err(InvariantError::DependsOnSelf(name.to_string()));
                }
            }
        }
        Ok(path)
    }

    /// Checks if the given invariant path actually leads to the executable
    fn get_path(exec_path: String) -> Result<PathBuf, InvariantError> {
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
            // Try to turn it into an absol
            return Err(InvariantError::InvalidPath(inv_path.to_path_buf()));
        }
        if !inv_path.is_executable() {
            return Err(InvariantError::NotExecutable(inv_path.to_path_buf()));
        }
        Ok(inv_path.to_path_buf())
    }

    /// Checks if the given invariant name can be used to create a table and/or a column in a database
    ///
    /// # Errors :
    /// * If the given name is not ascii
    /// * If the given name does not match with the following regex: [`INVARIANT_REGEX`]
    pub fn check_invariant_name_validity(name: &String) -> Result<(), InvariantError> {
        let re = Regex::new(INVARIANT_REGEX).expect("Regex should be okay");
        if !name.is_ascii() || !re.is_match(name) {
            return Err(InvariantError::InvalidName(name.to_string()));
        }
        Ok(())
    }

    /// Execute this executable by feeding it the given `input_buffer` into its *stdin* and sending every output read to the given `output_function`
    /// # Errrors
    /// Returns an [`InvariantExecutionError`] if something goes wrong during the execution of the process.
    pub async fn execute_invariant<T, F, E>(
        &self,
        input_function: &mut T,
        output_function: &mut F,
    ) -> Result<(), InvariantExecutionError>
    where
        T: AsyncFnMut() -> Option<Result<String, E>>,
        F: AsyncFnMut(String),
    {
        debug!("Start executable : {}", self.exec_path.display());
        let mut call_res = match Command::new(self.exec_path.as_os_str())
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(res) => res,
            Err(e) => return Err(InvariantExecutionError::FailedExecution(e.to_string())),
        };

        let mut stderr = call_res.stderr.take().expect("stdout to be open");

        // This block forces to close the opened stdin and stdout before
        // waiting for the child process to exit.
        // Otherwise a deadlock might appear
        let mut stdout = call_res.stdout.take().expect("stdout to be open");
        
        let mut waiting_in_stdin = 0;
        {
            let mut stdin = call_res.stdin.take().expect("stdin to be open");
            // For every value to send
            while let Some(Ok(val)) = input_function().await {
                // Check if the child closed or not during the execution
                self.check_child_state(&mut call_res, &mut stderr)?;

                // Push this value to content
                debug!("Wrote to child stdin: {val}");
                self.exec_stdin_io_call(&mut || stdin.write(format!("{val}\n").as_bytes()))?;
                waiting_in_stdin += 1;

                // Send value to buffer
                if waiting_in_stdin >= MAX_STDIN_SIZE {
                    self.exec_stdin_io_call(&mut || stdin.flush())?;
                    self.flush_wait_output(
                        &mut call_res,
                        waiting_in_stdin,
                        output_function,
                        &mut stdout,
                        &mut stderr,
                    )
                    .await?;
                    waiting_in_stdin = 0;
                }
            }
            self.exec_stdin_io_call(&mut || stdin.flush())?;
        }
        // If there are still data to send
        if waiting_in_stdin > 0 {
            self.flush_wait_output(
                &mut call_res,
                waiting_in_stdin,
                output_function,
                &mut stdout,
                &mut stderr,
            )
            .await?;
        }
        debug!("Finished child execution : {}", self.exec_path.display());

        // Check if the child closed or not during the execution
        self.check_child_state(&mut call_res, &mut stderr)?;

        Ok(())
    }

    async fn flush_wait_output<T>(
        &self,
        child: &mut Child,
        sent_in_stdin: usize,
        output_function: &mut T,
        stdout: &mut ChildStdout,
        stderr: &mut ChildStderr,
    ) -> Result<(), InvariantExecutionError>
    where
        T: AsyncFnMut(String),
    {
        debug!("Flusing stdin then waiting for {sent_in_stdin} responses");

        // A buffer helps us to not read all lines at the time but as a flow of data.
        let child_output = BufReader::new(stdout);

        let mut received = 0;

        for response in child_output.lines() {
            debug!("Received : {response:?}");
            // We are waiting for the exact number of data sent to be sent back to us.
            if let Ok(s) = response {
                output_function(s).await;
            }

            received += 1;
            if received == sent_in_stdin {
                break;
            }
        }
        debug!("Finished waiting");
        // If the stdout finished *before* receiving all the values sent
        // then it means the child probably crashed.
        if received != sent_in_stdin {
            // We loop multiple time because sometimes the stdin closes before the program
            // has actually the time to write to the stderr and close itself
            loop {
                self.check_child_state(child, stderr)?
            }
        } else {
            Ok(())
        }
    }

    /// Checks if the child is closed or not.
    /// # Errors :
    /// If the program was closed and an error code is returned, then a
    ///  [`InvariantExecutionError::EarlyExit`] with the error read from the `stderr` will be returned.
    fn check_child_state(
        &self,
        child: &mut Child,
        stderr: &mut ChildStderr,
    ) -> Result<(), InvariantExecutionError> {
        if let Ok(Some(status)) = child.try_wait() {
            if status.success() {
                return Ok(());
            }
            let child_buffer_stderr = BufReader::new(stderr);
            let mut err = String::new();
            for mut c in child_buffer_stderr.lines().map_while(Result::ok) {
                c.push('\n');
                err.push_str(&c);
            }
            Err(InvariantExecutionError::EarlyExit(
                self.exec_path.display().to_string(),
                status.to_string(),
                err,
            ))
        } else {
            Ok(())
        }
    }

    fn exec_stdin_io_call<T>(
        &self,
        to_call: &mut dyn FnMut() -> Result<T, io::Error>,
    ) -> Result<T, InvariantExecutionError> {
        match to_call() {
            Ok(t) => Ok(t),
            Err(e) => Err(InvariantExecutionError::FailedWriteStdin(
                e,
                self.exec_path.display().to_string(),
            )),
        }
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

    /// Adds an invariant executable to the sorter.
    pub fn add_inv_exec(&mut self, inv: InvariantsExecutable) -> Result<(), InvariantError> {
        if self.path_exec_hashmap.contains_key(&inv.exec_path) {
            return Err(InvariantError::AlreadyAddedExecutable(
                inv.exec_path.clone(),
            ));
        }
        for name in &inv.invariant_names {
            if self.name_path_hashmap.contains_key(name) {
                return Err(InvariantError::AlreadyAddedInvariant(name.to_string()));
            }
            self.name_path_hashmap
                .insert(name.to_string(), inv.exec_path.clone());
        }
        self.path_exec_hashmap.insert(inv.exec_path.clone(), inv);
        Ok(())
    }

    /// Adds multiple invariant executables to the sorter.
    pub fn add_inv_execs(
        &mut self,
        execs: Vec<InvariantsExecutable>,
    ) -> Result<(), InvariantError> {
        for inv in execs {
            self.add_inv_exec(inv)?;
        }
        Ok(())
    }

    /// Performs a topological sort with the stored [`InvariantsExecutable`]s
    /// # Errors
    /// * [`InvariantError::MissingDependency`] if one of the dependencies from one invariant is not present.
    /// * [`InvariantError::DependencyCycle`] if a cycle is found.
    pub fn sort(mut self) -> Result<Vec<InvariantsExecutable>, InvariantError> {
        // Init the topological sort
        let mut topo_sort: TopoSort<PathBuf> =
            TopoSort::with_capacity(self.path_exec_hashmap.len());

        // Add nodes
        for (path, exec) in &self.path_exec_hashmap {
            // Check that all dependencies of this executable are present
            // And gather all executable path that this executable is relying on
            let mut dependencies: Vec<PathBuf> = vec![];
            if let Some(deps) = &exec.dependencies {
                for dep in deps {
                    match self.name_path_hashmap.get(dep) {
                        Some(path) => {
                            dependencies.push(path.clone());
                        }
                        None => {
                            return Err(InvariantError::MissingDependency(
                                dep.to_string(),
                                exec.exec_path.clone(),
                            ));
                        }
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
                return Err(InvariantError::DependencyCycle);
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

    pub fn group_execs(self) -> Result<ExecutableManager, InvariantError> {
        let sorted_execs = self.sort()?;

        let mut res = ExecDepStore::default();
        let mut name_index_hash: HashMap<String, usize> = HashMap::new();

        for (index, exec) in sorted_execs.into_iter().enumerate() {
            for name in &exec.invariant_names {
                name_index_hash.insert(name.clone(), index); // insert into hashmap for an easy access to his index
            }

            // Add dependencies
            if let Some(dependencies) = &exec.dependencies {
                for dep_name in dependencies {
                    // By definition of a topological sort,
                    // these dependencies are from previous executables
                    // and therefore are in the name hashmap
                    res.dep_indexes[*name_index_hash.get(dep_name).expect("should be present")]
                        .push(index)
                }
                res.dep_left.push(dependencies.len());
            } else {
                res.dep_left.push(0);
            }

            res.executables.push(Some(exec));
            res.dep_indexes.push(vec![]);
        }
        Ok(res.group_processes())
    }
}

impl TryFrom<Vec<InvariantsExecutable>> for ExecutableSorter {
    type Error = InvariantError;

    fn try_from(values: Vec<InvariantsExecutable>) -> Result<Self, Self::Error> {
        let mut sorter = Self::new();
        for val in values {
            sorter.add_inv_exec(val)?;
        }

        Ok(sorter)
    }
}

impl TryFrom<Vec<&InvariantsExecutable>> for ExecutableSorter {
    type Error = InvariantError;

    fn try_from(values: Vec<&InvariantsExecutable>) -> Result<Self, Self::Error> {
        let mut sorter = Self::new();
        for val in values {
            sorter.add_inv_exec(val.clone())?;
        }

        Ok(sorter)
    }
}

#[derive(Debug, Default)]
/// Used to facilitate the creation of a [`ExecutableManager`] instance.
struct ExecDepStore {
    /// The invariants sorted using a topological sort
    executables: Vec<Option<InvariantsExecutable>>,
    /// The number of dependencies left for an invariants
    dep_left: Vec<usize>,
    /// The indexes of the executables that depend on this executable.
    dep_indexes: Vec<Vec<usize>>,
}

impl ExecDepStore {
    fn group_processes(mut self) -> ExecutableManager {
        let mut manager: ExecutableManager = ExecutableManager::default();
        let mut to_sort = self.executables.len();
        // While not all execs are sorted
        while to_sort > 0 {
            let mut can_now_exec: Vec<InvariantsExecutable> = vec![];
            let mut can_now_exec_ind: Vec<usize> = vec![];

            // Get all processes that can now be executed
            // this is what we refer to as a "group"
            for i in (0..self.executables.len()).rev() {
                if self.dep_left[i] == 0 && self.executables[i].is_some() {
                    can_now_exec.push(self.executables[i].take().expect("present"));
                    can_now_exec_ind.push(i);
                    to_sort -= 1;
                }
            }

            manager.groups.push(can_now_exec);

            // Update dependencies
            can_now_exec_ind.iter().for_each(|i| {
                for dep_index in &self.dep_indexes[*i] {
                    self.dep_left[*dep_index] -= 1;
                }
            });
        }
        manager
    }
}

// FIXME: Add more information to this doc
/// This structs holds a [`Vec<Vec<InvariantsExecutable>>`],
/// * with the outer vector representing a topological sort
/// * the middle vector containing a vector of executable that could be executed simultaneously.
/// * and the inner vector grouping invariants that have the same dependencies.
///  
/// # Explanation using an example
/// Syntax :
/// - `->` : *depends on*
/// - `=>` : *then execute*
///
/// Let 5 processes `[A, B, C, D, E]`, with the following dependencies (for example B1 means a dependency from the executable B) :
/// * `[A -> B1, C1; B -> /; C -> D1; D -> B1; E -> /]`
///
/// A topological order could be:
/// * `E => B => A => D => C`.
///
/// But since they don't all depend on each others and sometimes have the same dependencies, we could execute multiple ones at the same time like
/// **E** and **B**, **A** and **D**. So the order of execution could be
/// * `[E, B] => [A, D] => [C]`
///
/// Meaning we now have 3 execution groups, but we also have to think about the max number of process allowed `m`.
/// If `m = 1` then we would have the following execution order
/// * `[ [E] => [B] ] => [ [A] => [D] ] => [ [C] ]`
///
/// Which is the orginal topological order.
#[derive(Debug, Clone, Default)]
pub struct ExecutableManager {
    groups: Vec<Vec<InvariantsExecutable>>,
}

impl ExecutableManager {
    pub fn get_groups(&self) -> &Vec<Vec<InvariantsExecutable>> {
        &self.groups
    }

    pub fn as_vec(self) -> Vec<Vec<InvariantsExecutable>> {
        self.groups
    }
}

impl Display for InvariantsExecutable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut inv_list = "{".to_string();

        for inv in &self.invariant_names {
            inv_list.push_str(inv);
            inv_list.push_str(", ")
        }
        inv_list.pop();
        inv_list.pop();
        inv_list.push('}');
        write!(f, "{}", inv_list)
    }
}

impl Display for ExecutableManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = "[".to_string();
        let sep = " => ";
        for group in &self.groups {
            let mut group_str = "[".to_string();
            for inv in group {
                group_str.push_str(&inv.to_string());
                group_str.push_str(", ");
            }
            group_str.pop();
            group_str.pop();
            group_str.push(']');
            group_str.push_str(sep);

            res.push_str(&group_str);
        }
        res.pop();
        res.pop();
        res.pop();
        res.pop();
        res.push(']');

        write!(f, "{res}")
    }
}
