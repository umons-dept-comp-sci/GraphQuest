use indexmap::IndexMap;
use is_executable::IsExecutable;
use log::debug;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::{Debug, Display},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdout, Command, Stdio},
};
use thiserror::Error;
use topo_sort::TopoSort;

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

#[derive(Debug, Error)]
pub enum ModuleError {
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
    #[error("Tried to add the following module \"{0}\" twice")]
    AlreadyAddedModule(PathBuf),
    #[error("Tried to add an invariant with the name \"{0}\" twice")]
    AlreadyAddedInvariant(String),
    #[error("Encountered a dependency cycle, thus making the invariant computation impossible")]
    DependencyCycle,
}

/// Represent an executable file that can be used to compute graph invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    /// The *absolute* path to the file to execute.
    pub exec_path: PathBuf,

    /// The name of the computed invariants in the returned order
    pub invariant_names: Vec<String>,

    /// The vector of dependencies requiered to compute this invariant.
    /// A dependency cannot be present in the invariant names field.
    pub dependencies: Option<Vec<String>>,
}

/// Trait used to provide a stream of data to provided to a module
pub trait AsyncModuleInput<E> {
    /// Returns the next values to feed to the invariant. None if everything was already given.
    fn call(&mut self) -> impl std::future::Future<Output = Option<Result<Vec<String>, E>>> + Send;
}

/// Trait used to save the data returned from the invariants.
pub trait AsyncModuleOutput<E> {
    /// Provides the invariant returned values.
    fn call(
        &mut self,
        values: Vec<String>,
    ) -> impl std::future::Future<Output = Result<(), E>> + Send;
}

impl Module {
    /// Creates an invariant using the given parameters
    pub fn new(
        exec_path: impl Into<String>,
        names: Vec<impl Into<String>>,
        dependencies: Vec<impl Into<String>>,
    ) -> Result<Self, ModuleError> {
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

        let i = Module {
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
    ) -> Result<Self, ModuleError> {
        let invariant_names: Vec<String> = names.into_iter().map(|n| n.into()).collect();

        let exec_path = Self::check_validity(exec_path.into(), &invariant_names, None)?;

        let i = Module {
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
    ) -> Result<PathBuf, ModuleError> {
        let path = Self::get_path(exec_path)?;
        for name in names {
            Self::check_invariant_name_validity(name)?;
            if let Some(dependencies) = dependencies
                && dependencies.contains(name)
            {
                return Err(ModuleError::DependsOnSelf(name.to_string()));
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
    pub fn check_invariant_name_validity(name: &String) -> Result<(), ModuleError> {
        let re = Regex::new(INVARIANT_REGEX).expect("Regex should be okay");
        if !name.is_ascii() || !re.is_match(name) {
            return Err(ModuleError::InvalidName(name.to_string()));
        }
        Ok(())
    }

    /// Execute this module by feeding it the given `input_buffer` into its *stdin* and sending every output read to the given `output_function`
    /// # Errrors
    /// Returns an [`ModuleExecutionError`] if something goes wrong during the execution of the process.
    pub async fn execute<T, F, E>(
        &self,
        mut input_function: T,
        mut output_function: F,
        batch_size: usize,
    ) -> Result<(), ModuleExecutionError>
    where
        T: AsyncModuleInput<E>,
        F: AsyncModuleOutput<E>,
        E: Debug,
    {
        debug!("Starts module : {}", self.exec_path.as_os_str().display());
        let mut call_res = match Command::new(self.exec_path.as_os_str())
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(res) => res,
            Err(e) => return Err(ModuleExecutionError::FailedExecution(e.to_string())),
        };

        let Some(mut stderr) = call_res.stderr.take() else {
            return Err(ModuleExecutionError::ClosedStream(
                "stderr".to_string(),
                self.exec_path.display().to_string(),
            ));
        };

        let Some(mut stdout) = call_res.stdout.take() else {
            return Err(ModuleExecutionError::ClosedStream(
                "stdout".to_string(),
                self.exec_path.display().to_string(),
            ));
        };

        let mut waiting_in_stdin = 0;

        // This block forces to close the opened stdin before
        // waiting for the child process to exit.
        // Otherwise a deadlock might appear because the child didn't flush in time.
        {
            let Some(mut stdin) = call_res.stdin.take() else {
                return Err(ModuleExecutionError::ClosedStream(
                    "stdin".to_string(),
                    self.exec_path.display().to_string(),
                ));
            };

            // For every value to send
            while let Some(val) = input_function.call().await {
                let val = match val {
                    Ok(val) => val,
                    Err(e) => {
                        return Err(ModuleExecutionError::FailedInput(format!("{e:?}")));
                    }
                };

                // Check if the child closed or not during the execution
                self.check_child_state(&mut call_res, &mut stderr)?;

                // Push this value to content
                if let Some(dep) = &self.dependencies
                    && dep.len() + 1 != val.len()
                {
                    return Err(ModuleExecutionError::UnexpectedInput(
                        self.exec_path.display().to_string(),
                        val.len(),
                        dep.len() + 1,
                    ));
                }
                let val = val.join(" ");
                debug!("Wrote to child stdin ({:?}): {val:?}", self.invariant_names);
                self.exec_stdin_io_call(&mut || stdin.write(format!("{val}\n").as_bytes()))?;
                waiting_in_stdin += 1;

                // Send value to buffer
                if waiting_in_stdin >= batch_size {
                    self.exec_stdin_io_call(&mut || stdin.flush())?;
                    self.flush_wait_output(
                        &mut call_res,
                        waiting_in_stdin,
                        &mut output_function,
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
                &mut output_function,
                &mut stdout,
                &mut stderr,
            )
            .await?;
        }
        debug!(
            "Finished child execution ({:?}): {}",
            self.invariant_names,
            self.exec_path.display()
        );

        // Check if the child closed or not during the execution
        self.check_child_state(&mut call_res, &mut stderr)?;

        Ok(())
    }

    async fn flush_wait_output<T, E>(
        &self,
        child: &mut Child,
        sent_in_stdin: usize,
        output_function: &mut T,
        stdout: &mut ChildStdout,
        stderr: &mut ChildStderr,
    ) -> Result<(), ModuleExecutionError>
    where
        T: AsyncModuleOutput<E>,
        E: Debug,
    {
        debug!(
            "Flusing stdin then waiting for {sent_in_stdin} responses ({:?})",
            self.invariant_names
        );

        // A buffer helps us to not read all lines at the time but as a flow of data.
        let child_output = BufReader::new(stdout);

        let mut received = 0;

        for response in child_output.lines() {
            debug!("Received ({:?}): {response:?}", self.invariant_names);
            // We are waiting for the exact number of data sent to be sent back to us.
            if let Ok(s) = response {
                let vals: Vec<String> = s.split(' ').map(|v| v.to_string()).collect();
                // Should return the signature and a value for each invariants
                if vals.len() != self.invariant_names.len() + 1 {
                    return Err(ModuleExecutionError::UnexpectedOutput(
                        self.exec_path.display().to_string(),
                        vals.len(),
                        self.invariant_names.len() + 1,
                    ));
                }
                if let Err(e) = output_function.call(vals).await {
                    return Err(ModuleExecutionError::FailedOutput(format!("{e:?}")));
                }
            }

            received += 1;
            if received == sent_in_stdin {
                break;
            }
        }
        debug!("Finished waiting ({:?})", self.invariant_names);
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
    ///  [`ModuleExecutionError::EarlyExit`] with the error read from the `stderr` will be returned.
    fn check_child_state(
        &self,
        child: &mut Child,
        stderr: &mut ChildStderr,
    ) -> Result<(), ModuleExecutionError> {
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
            Err(ModuleExecutionError::EarlyExit(
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
    ) -> Result<T, ModuleExecutionError> {
        match to_call() {
            Ok(t) => Ok(t),
            Err(e) => Err(ModuleExecutionError::FailedWriteStdin(
                e,
                self.exec_path.display().to_string(),
            )),
        }
    }
}

/// Stores [`Module`]s in order to prepare a topology sort.
/// Such as by checking :
/// * if an invariant was already added
/// * if one of it's dependencies does not exists
#[derive(Default, Clone, Debug)]
pub struct ModuleSorter {
    /// `Invariant name` -> `Linked Executable path`
    name_path_hashmap: IndexMap<String, PathBuf>,
    /// `Executable path` -> `Module`
    path_exec_hashmap: IndexMap<PathBuf, Module>,
}

impl ModuleSorter {
    /// Creates a new empty [`ModuleSorter`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Using the given invariant names, only adds the executables that computes them from the given array of executables.
    /// # Errors
    /// Returns an [`ModuleError::UnknownInvariant`] if one of the given invariant name was not present in any of the executables from the given array.
    pub fn new_from(
        all_invariants: &[Module],
        inv_to_add: &HashSet<impl ToString>,
    ) -> Result<Self, ModuleError> {
        let mut res = Self::new();

        // To ease the process, creates an hashmap : `name` -> `index of execs`
        let mut name_index_map: HashMap<&String, usize> = HashMap::new();
        for (i, execs) in all_invariants.iter().enumerate() {
            for inv_name in &execs.invariant_names {
                name_index_map.insert(inv_name, i);
            }
        }

        // Adds all invariants
        for inv_name in inv_to_add {
            res.add_inv_from_name(all_invariants, &name_index_map, &inv_name.to_string())?;
        }

        Ok(res)
    }

    /// Adds the given module using the given name.
    fn add_inv_from_name(
        &mut self,
        all_invariants: &[Module],
        name_index_map: &HashMap<&String, usize>,
        inv_to_add: &String,
    ) -> Result<(), ModuleError> {
        let Some(exec_index) = name_index_map.get(inv_to_add) else {
            return Err(ModuleError::UnknownInvariant(inv_to_add.to_string()));
        };

        let exec = &all_invariants[*exec_index];

        // Try to add invariant if not already added
        if let Err(e) = self.add_inv_exec(exec.clone()) {
            if let ModuleError::AlreadyAddedModule(_) = e {
            } else {
                return Err(e);
            }
        }

        // Adds all dependencies (recursively)
        if let Some(dependencies) = &exec.dependencies {
            for dep in dependencies {
                self.add_inv_from_name(all_invariants, name_index_map, dep)?;
            }
        }

        Ok(())
    }

    /// Adds an invariant executable to the sorter.
    pub fn add_inv_exec(&mut self, inv: Module) -> Result<(), ModuleError> {
        if self.path_exec_hashmap.contains_key(&inv.exec_path) {
            return Err(ModuleError::AlreadyAddedModule(inv.exec_path.clone()));
        }
        for name in &inv.invariant_names {
            if self.name_path_hashmap.contains_key(name) {
                return Err(ModuleError::AlreadyAddedInvariant(name.to_string()));
            }
            self.name_path_hashmap
                .insert(name.to_string(), inv.exec_path.clone());
        }
        self.path_exec_hashmap.insert(inv.exec_path.clone(), inv);
        Ok(())
    }

    /// Adds multiple invariant executables to the sorter.
    pub fn add_modules(&mut self, execs: Vec<Module>) -> Result<(), ModuleError> {
        for inv in execs {
            self.add_inv_exec(inv)?;
        }
        Ok(())
    }

    /// Returns the number of executable present inside this sorter.
    #[must_use]
    pub fn len(&self) -> usize {
        self.path_exec_hashmap.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.path_exec_hashmap.is_empty()
    }

    /// Performs a topological sort with the stored [`Module`]s
    /// # Errors
    /// * [`ModuleError::MissingDependency`] if one of the dependencies from one invariant is not present.
    /// * [`ModuleError::DependencyCycle`] if a cycle is found.
    pub fn sort(mut self) -> Result<Vec<Module>, ModuleError> {
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
                            return Err(ModuleError::MissingDependency(
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
                return Err(ModuleError::DependencyCycle);
            }
        };

        let mut res = vec![];
        for path in ordered_paths {
            res.push(
                self.path_exec_hashmap
                    .swap_remove(&path)
                    .expect("present in hashmap"),
            );
        }

        Ok(res)
    }

    /// Consumes the sorter and turns it into an [`ModuleIterator`]
    /// # Errors
    /// See the [`ModuleSorter::sort`] method.
    pub fn to_iter(self) -> Result<ModuleIterator, ModuleError> {
        let sorted_execs = self.sort()?;

        let mut res = ExecDepStore::default();
        let mut name_index_hash: IndexMap<String, usize> = IndexMap::new();

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

            res.modules.push(Some(exec));
            res.dep_indexes.push(vec![]);
        }
        Ok(ModuleIterator::new(res))
    }
}

#[derive(Debug)]
pub struct ModuleIterator {
    store: ExecDepStore,
    can_now_exec: Vec<Module>,
    being_computed: IndexMap<PathBuf, usize>,
    left_to_exec: usize,
}

impl ModuleIterator {
    fn new(store: ExecDepStore) -> Self {
        let mut res = Self {
            left_to_exec: store.modules.len(),
            can_now_exec: vec![],
            being_computed: IndexMap::new(),
            store,
        };
        res.update_can_now_exec();
        res
    }

    pub fn has_next(&self) -> bool {
        !self.can_now_exec.is_empty()
    }

    pub fn is_finished(&self) -> bool {
        self.left_to_exec == 0
    }

    pub fn update_dependencies(&mut self, inv_exec: &Module) {
        // Find the given invariant (if it is even present)
        if let Some(index) = self.being_computed.swap_remove(&inv_exec.exec_path) {
            for dep_index in &self.store.dep_indexes[index] {
                // Prevents any overflow crashes
                if self.store.dep_left[*dep_index] > 0 {
                    self.store.dep_left[*dep_index] -= 1;
                }
            }
            self.update_can_now_exec();
        }
    }

    fn update_can_now_exec(&mut self) {
        // Get all processes that can now be executed
        for i in (0..self.store.modules.len()).rev() {
            if self.store.dep_left[i] == 0 && self.store.modules[i].is_some() {
                let can_exec = self.store.modules[i].take().expect("present");
                self.being_computed.insert(can_exec.exec_path.clone(), i);
                self.can_now_exec.push(can_exec);
            }
        }
    }

    pub fn next_module(&mut self) -> Option<Module> {
        // If an invariant is waiting
        if !self.can_now_exec.is_empty() {
            self.left_to_exec -= 1;
            return Some(self.can_now_exec.pop().expect("not empty"));
        }
        None
    }
}

impl Iterator for ModuleIterator {
    type Item = Module;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inv) = self.next_module() {
            self.update_dependencies(&inv);
            Some(inv)
        } else {
            None
        }
    }
}

impl TryFrom<Vec<Module>> for ModuleSorter {
    type Error = ModuleError;

    fn try_from(values: Vec<Module>) -> Result<Self, Self::Error> {
        let mut sorter = Self::new();
        for val in values {
            sorter.add_inv_exec(val)?;
        }

        Ok(sorter)
    }
}

impl TryFrom<Vec<&Module>> for ModuleSorter {
    type Error = ModuleError;

    fn try_from(values: Vec<&Module>) -> Result<Self, Self::Error> {
        let mut sorter = Self::new();
        for val in values {
            sorter.add_inv_exec(val.clone())?;
        }

        Ok(sorter)
    }
}

#[derive(Debug, Default)]
/// Used to facilitate the creation of a [`ModuleIterator`] instance.
/// It is not meant to be used publically.
struct ExecDepStore {
    /// The invariants sorted using a topological sort
    modules: Vec<Option<Module>>,
    /// The number of dependencies left for an invariants
    dep_left: Vec<usize>,
    /// The indexes of the executables that depend on this executable.
    dep_indexes: Vec<Vec<usize>>,
}

impl Display for Module {
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
