use is_executable::IsExecutable;
use log::debug;
use regex::Regex;
use std::{
    collections::{HashMap, hash_map::Entry},
    env,
    fmt::{Debug, Display},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdout, Command, Stdio},
};
use thiserror::Error;

use crate::{
    data_handler::{data_types::ValueType, rel_graph::FnRef},
    parser::parsed_expression::MathExpression,
};

const INVARIANT_REGEX: &str = "^([a-z]|[A-Z]|_)(_|[a-z]|[A-Z]|[0-9])*$";

#[derive(Debug, Error)]
pub enum ModuleError {
    #[error("Any module must require at least one parameter")]
    NoArgs,
    #[error("The given path \"{0}\" does not lead to a file")]
    InvalidPath(PathBuf),
    #[error("The given file at \"{0}\" is not executable")]
    NotExecutable(PathBuf),
    #[error(
        "The given name \"{0}\" is not a valid function name. An function name must follow the following regex : ^([a-z]|[A-Z]|_)(_|[a-z]|[A-Z]|[0-9])*$"
    )]
    InvalidName(String),
    #[error("The module function has the same name as one of needed argument: \"{0}\"")]
    ReturnSelf(String),
    #[error("Encountered an IoError : \"{0}\"")]
    IoError(#[from] io::Error),
}

#[derive(Debug, Error)]
pub enum ModuleLoaderError {
    UnknownTemplateError {
        fn_name: String,
    },
    UnknownLoadedModuleError {
        fn_name: String,
    },
    LoadError {
        fn_name: String,
        path: String,
        reason: ModuleError,
    },
}

impl Display for ModuleLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", {
            match self {
                ModuleLoaderError::UnknownTemplateError { fn_name } => {
                    format!("No module template exists with the given function name \"{fn_name}\"")
                }
                ModuleLoaderError::UnknownLoadedModuleError { fn_name } => {
                    format!("No loaded module has the current function name \"{fn_name}\"")
                }
                ModuleLoaderError::LoadError {
                    fn_name,
                    path,
                    reason,
                } => {
                    format!(
                        "Failed to load the module with the function name \"{fn_name}\", with the given path \"{path}\", because \"{reason}\"",
                    )
                }
            }
        })
    }
}

#[derive(Debug, Error)]
pub struct ModuleExecError {
    pub fn_name: String,
    pub path: String,
    pub reason: ExecErrorReason,
}

impl Display for ModuleExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The module computing function \"{}\" (at path:\"{}\") ran into an issue: {}",
            self.fn_name, self.path, self.reason
        )
    }
}

#[derive(Debug, Error)]
pub enum ExecErrorReason {
    #[error("Failed to start the execution of the module: \"{0}\"")]
    FailedExecution(io::Error),
    #[error("Failed to write to the stdin of the module: \"{0}\"")]
    FailedWriteStdin(io::Error),
    #[error("Failed to open the {0} of module")]
    ClosedStream(String),
    #[error(
        "The module finished its execution earlier than expected : Exit status \"{0}\" | stderr : \n\"{1}\" "
    )]
    EarlyExit(String, String),
    #[error("The module returned \"{0}\" values instead of \"{1}\"")]
    UnexpectedOutput(usize, usize),
    #[error("Tried to input \"{0}\" values instead of \"{1}\" to the module")]
    UnexpectedInput(usize, usize),
    #[error("The output function returned an error during the execution : \"{0}\"")]
    FailedOutput(String),
    #[error("The input function returned an error during the execution : \"{0}\"")]
    FailedInput(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedArg {
    pub name: String,
    pub data_type: ValueType,
}

impl TypedArg {
    pub fn has_same_name(&self, other: &TypedArg) -> bool {
        self.name == other.name
    }
}

impl From<(String, ValueType)> for TypedArg {
    fn from(value: (String, ValueType)) -> Self {
        Self {
            name: value.0,
            data_type: value.1,
        }
    }
}

impl From<(&str, ValueType)> for TypedArg {
    fn from(value: (&str, ValueType)) -> Self {
        (value.0.to_string(), value.1).into()
    }
}

/// Contains the set of available modules. Is used to only try to load modules when they are neeeded.
#[derive(Default, Debug)]
pub struct ModuleLoader {
    templates: HashMap<String, ModuleTemplate>,
    loaded: HashMap<String, Module>,
}

impl ModuleLoader {
    /// Adds a module to the templates.
    /// If a previously added function has the same name, the new module is not added
    /// and the previous ones path is returned, otherwise the module is added and nothing is returned.
    pub fn add_module_template(&mut self, template: ModuleTemplate) -> Option<String> {
        match self.templates.entry(template.fn_name.to_owned()) {
            Entry::Occupied(entry) => Some(entry.into_mut().exec_path.to_string()),
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(template);
                None
            }
        }
    }

    /// Get the hashmap containing all module templates stored.
    pub fn get_templates(&self) -> &HashMap<String, ModuleTemplate> {
        &self.templates
    }

    /// Tries to find the module with the given function name.
    ///
    /// # Errors:
    /// * [`ModuleLoaderError::UnknownLoadedModuleError`] if the function name is not the name of any loaded module.
    pub fn try_get_loaded_module(&self, fn_name: &str) -> Result<&Module, ModuleLoaderError> {
        match self.loaded.get(fn_name) {
            Some(module) => Ok(module),
            None => Err(ModuleLoaderError::UnknownLoadedModuleError {
                fn_name: fn_name.to_owned(),
            }),
        }
    }

    /// Tries to load a module and returns its reference.
    ///
    /// # Errors:
    /// if no module with the given name were loaded and if
    /// * no template has the given function name: [`ModuleLoaderError::UnknownTemplateError`],
    /// * the template could not be turned into an instance of a module: [`ModuleLoaderError::LoadError`].
    pub fn load_module(&mut self, fn_name: &str) -> Result<&Module, ModuleLoaderError> {
        match self.loaded.entry(fn_name.to_owned()) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),

            Entry::Vacant(entry) => {
                let template = self.templates.get(fn_name).ok_or_else(|| {
                    ModuleLoaderError::UnknownTemplateError {
                        fn_name: fn_name.to_string(),
                    }
                })?;

                let module = Module::from_template(template).map_err(|reason| {
                    ModuleLoaderError::LoadError {
                        fn_name: fn_name.to_owned(),
                        path: template.exec_path.to_string(),
                        reason,
                    }
                })?;

                Ok(entry.insert(module))
            }
        }
    }
}

/// Represents an unloaded module, meaning the data contained here can be used to try to initialised a module when needed.
#[derive(Clone, Debug, PartialEq)]
pub struct ModuleTemplate {
    exec_path: String,
    fn_name: String,
    args: Vec<TypedArg>,
    add_args: Vec<MathExpression>,
    output: ValueType,
    batch_size: Option<usize>,
}

impl ModuleTemplate {
    pub fn new(
        exec_path: impl Into<String>,
        fn_name: impl Into<String>,
        args: Vec<impl Into<TypedArg>>,
        add_args: Vec<MathExpression>,
        output: ValueType,
        batch_size: Option<usize>,
    ) -> Self {
        Self {
            args: args.into_iter().map(|arg| arg.into()).collect(),
            batch_size,
            add_args,
            exec_path: exec_path.into(),
            fn_name: fn_name.into(),
            output,
        }
    }

    /// Creates a module with only one argument, a graph.
    /// Useful when making modules that act as invariants.
    /// The argument will be called `graph` and be a [`ValueType::Graph`].
    pub fn new_invariant(
        exec_path: impl Into<String>,
        fn_name: impl Into<String>,
        output: ValueType,
        batch_size: Option<usize>,
    ) -> Self {
        Self {
            args: Vec::from([("graph", ValueType::Graph).into()]),
            batch_size,
            add_args: Vec::new(),
            exec_path: exec_path.into(),
            fn_name: fn_name.into(),
            output,
        }
    }
}

/// Represent an executable file that can be used to compute graph invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    /// The *absolute* path to the module.
    pub exec_path: PathBuf,
    pub fn_name: String,
    pub args: Vec<TypedArg>,
    pub add_args: Vec<MathExpression>,
    /// The type of output to return
    pub output: ValueType,
    pub batch_size: Option<usize>,
}

impl Module {
    /// Tries to creates a module using the given template
    fn from_template(template: &ModuleTemplate) -> Result<Self, ModuleError> {
        let owned_template = template.clone();
        Self::new(
            owned_template.exec_path,
            owned_template.fn_name,
            owned_template.args,
            owned_template.add_args,
            owned_template.output,
            owned_template.batch_size,
        )
    }

    /// Tries to creates a module using the given parameters
    pub fn new(
        exec_path: impl Into<String>,
        fn_name: impl Into<String>,
        args: Vec<impl Into<TypedArg>>,
        add_args: Vec<MathExpression>,
        output: ValueType,
        batch_size: Option<usize>,
    ) -> Result<Self, ModuleError> {
        if args.is_empty() {
            return Err(ModuleError::NoArgs);
        }
        let fn_name: String = fn_name.into();
        let args: Vec<TypedArg> = args.into_iter().map(|n| n.into()).collect();

        let exec_path = Self::check_validity(exec_path.into(), &fn_name, &args)?;

        let i = Module {
            exec_path,
            fn_name,
            output,
            args,
            add_args,
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
            add_args: Vec::new(),
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
        args: &[TypedArg],
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
    pub fn check_invariant_name_validity(value: &TypedArg) -> Result<(), ModuleError> {
        let re = Regex::new(INVARIANT_REGEX).expect("Regex should be okay");
        if !value.name.is_ascii() || !re.is_match(&value.name) {
            return Err(ModuleError::InvalidName(value.name.to_string()));
        }
        Ok(())
    }
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
    /// Execute this module by feeding it the given `input_buffer` into its *stdin* and sending every output read to the given `output_function`
    /// # Errors
    /// Returns an [`ModuleExecutionError`] if something goes wrong during the execution of the process.
    pub async fn execute<T, F, E>(
        &self,
        fn_ref: &FnRef,
        mut input_function: T,
        mut output_function: F,
        batch_size: usize,
    ) -> Result<(), ModuleExecError>
    where
        T: AsyncModuleInput<E>,
        F: AsyncModuleOutput<E>,
        E: Debug,
    {
        debug!(
            "Starts function {:?} at {}",
            fn_ref,
            self.exec_path.as_os_str().display()
        );
        let mut call_res = match Command::new(self.exec_path.as_os_str())
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(res) => res,
            Err(e) => return Err(self.get_exec_error(ExecErrorReason::FailedExecution(e))),
        };

        let Some(mut stderr) = call_res.stderr.take() else {
            return Err(self.get_exec_error(ExecErrorReason::ClosedStream("stderr".to_string())));
        };

        let Some(mut stdout) = call_res.stdout.take() else {
            return Err(self.get_exec_error(ExecErrorReason::ClosedStream("stdout".to_string())));
        };

        let mut waiting_in_stdin = 0;

        // This block forces to close the opened stdin before
        // waiting for the child process to exit.
        // Otherwise a deadlock might appear because the child didn't flush in time.
        {
            let Some(mut stdin) = call_res.stdin.take() else {
                return Err(self.get_exec_error(ExecErrorReason::ClosedStream("stdin".to_string())));
            };

            // For every value to send
            while let Some(val) = input_function.call().await {
                let val = match val {
                    Ok(val) => val,
                    Err(e) => {
                        return Err(
                            self.get_exec_error(ExecErrorReason::FailedInput(format!("{e:?}")))
                        );
                    }
                };

                // Check if the child closed or not during the execution
                self.check_child_state(&mut call_res, &mut stderr)?;

                // Push this value to content
                if self.args.len() != val.len() {
                    return Err(self.get_exec_error(ExecErrorReason::UnexpectedInput(
                        val.len(),
                        self.args.len(),
                    )));
                }
                let val = val.join(" ");
                debug!("Wrote to child stdin ({:?}): {val:?}", fn_ref);

                self.exec_stdin_io_call(&mut || stdin.write(format!("{val}\n").as_bytes()))?;
                waiting_in_stdin += 1;

                // Send value to buffer
                if waiting_in_stdin >= batch_size {
                    self.exec_stdin_io_call(&mut || stdin.flush())?;
                    self.flush_wait_output(
                        fn_ref,
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
                fn_ref,
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
            fn_ref,
            self.exec_path.display()
        );

        // Check if the child closed or not during the execution
        self.check_child_state(&mut call_res, &mut stderr)?;

        Ok(())
    }

    async fn flush_wait_output<T, E>(
        &self,
        fn_ref: &FnRef,
        child: &mut Child,
        sent_in_stdin: usize,
        output_function: &mut T,
        stdout: &mut ChildStdout,
        stderr: &mut ChildStderr,
    ) -> Result<(), ModuleExecError>
    where
        T: AsyncModuleOutput<E>,
        E: Debug,
    {
        debug!(
            "Flusing stdin then waiting for {sent_in_stdin} responses ({:?})",
            fn_ref
        );

        // A buffer helps us to not read all lines at the time but as a flow of data.
        let child_output = BufReader::new(stdout);

        let mut received = 0;

        for response in child_output.lines() {
            debug!("Received ({:?}): {response:?}", fn_ref);
            // We are waiting for the exact number of data sent to be sent back to us.
            if let Ok(s) = response {
                let vals: Vec<String> = s.split(' ').map(|v| v.to_string()).collect();
                // Should return the arguments it used to compute a value + the outputted value
                if vals.len() != self.args.len() + 1 {
                    return Err(self.get_exec_error(ExecErrorReason::UnexpectedOutput(
                        vals.len(),
                        self.args.len() + 1,
                    )));
                }
                if let Err(e) = output_function.call(vals).await {
                    return Err(
                        self.get_exec_error(ExecErrorReason::FailedOutput(format!("{e:?}")))
                    );
                }
            }

            received += 1;
            if received == sent_in_stdin {
                break;
            }
        }
        debug!("Finished waiting ({:?})", fn_ref);
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
    ) -> Result<(), ModuleExecError> {
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
            Err(self.get_exec_error(ExecErrorReason::EarlyExit(status.to_string(), err)))
        } else {
            Ok(())
        }
    }

    fn exec_stdin_io_call<T>(
        &self,
        to_call: &mut dyn FnMut() -> Result<T, io::Error>,
    ) -> Result<T, ModuleExecError> {
        match to_call() {
            Ok(t) => Ok(t),
            Err(e) => Err(self.get_exec_error(ExecErrorReason::FailedWriteStdin(e))),
        }
    }

    fn get_exec_error(&self, reason: ExecErrorReason) -> ModuleExecError {
        ModuleExecError {
            fn_name: self.fn_name.clone(),
            path: self.exec_path.display().to_string(),
            reason,
        }
    }
}
