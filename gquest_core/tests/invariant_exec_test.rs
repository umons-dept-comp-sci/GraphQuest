use std::{
    io::{BufRead, BufReader, Lines, Read},
    process::ChildStdout,
};

use gquest_core::data_handler::{
    data_loader::GengProcess,
    data_types::ValueType::{self},
    module::{AsyncModuleInput, AsyncModuleOutput, ExecErrorReason, Module, ModuleExecError},
};

const MAX_STDIN_SIZE: usize = 40;

pub const VALID_EXEC_IDENTITY: &str = "tests/modules/identity.py";
pub const LATE_FLUSH_EXEC: &str = "tests/modules/late_flush.py";
pub const CRASH_BEFORE_EXEC: &str = "tests/modules/crash_before.py";
pub const CRASH_DURING_EXEC: &str = "tests/modules/crash_during.py";

struct InputFn {
    reader: Lines<BufReader<ChildStdout>>,
}

impl AsyncModuleInput<()> for InputFn {
    async fn call(&mut self) -> Option<Result<Vec<String>, ()>> {
        let v = self.reader.next()?.ok()?;
        Some(Ok([v].to_vec()))
    }
}

struct OutputFn<'e> {
    output_buffer: &'e mut Vec<String>,
}

impl<'e> AsyncModuleOutput<()> for OutputFn<'e> {
    async fn call(&mut self, values: Vec<String>) -> Result<(), ()> {
        self.output_buffer.push(values[1].clone());
        Ok(())
    }
}

struct NoOutputFn {}

impl AsyncModuleOutput<()> for NoOutputFn {
    async fn call(&mut self, _values: Vec<String>) -> Result<(), ()> {
        Ok(())
    }
}

#[tokio::test]
async fn execute_correct_inv() {
    let identity = Module::new_invariant(
        VALID_EXEC_IDENTITY.to_string(),
        "identity",
        ValueType::Graph,
        None,
    )
    .expect("correct inv");

    let geng =
        GengProcess::call_geng(None, 5, &"".to_string(), (None, None)).expect("correct call");
    let mut res: String = String::default();
    geng.get_reader().read_to_string(&mut res).expect("correct");
    let expected_res: Vec<&str> = res.split_ascii_whitespace().collect();

    let geng =
        GengProcess::call_geng(None, 5, &"".to_string(), (None, None)).expect("correct call");
    let reader = geng.get_reader().lines();
    let input = InputFn { reader };

    let mut output_buffer = vec![];

    let output = OutputFn {
        output_buffer: &mut output_buffer,
    };

    identity
        .execute(&("identity".to_string(), 0), input, output, MAX_STDIN_SIZE)
        .await
        .expect("ok");

    assert_eq!(expected_res, output_buffer);
}

#[tokio::test]
async fn execute_late_inv() {
    let identity = Module::new_invariant(
        LATE_FLUSH_EXEC.to_string(),
        "late_flush",
        ValueType::Numeric,
        None,
    )
    .expect("correct inv");

    let geng =
        GengProcess::call_geng(None, 5, &"".to_string(), (None, None)).expect("correct call");
    let mut res: String = String::default();
    geng.get_reader().read_to_string(&mut res).expect("correct");
    let expected_res: Vec<&str> = res.split_ascii_whitespace().collect();

    let geng =
        GengProcess::call_geng(None, 5, &"".to_string(), (None, None)).expect("correct call");
    let reader = geng.get_reader().lines();
    let input = InputFn { reader };
    let mut actual_res: Vec<String> = vec![];

    let output = OutputFn {
        output_buffer: &mut actual_res,
    };

    identity
        .execute(
            &("late_flush".to_string(), 0),
            input,
            output,
            MAX_STDIN_SIZE,
        )
        .await
        .expect("ok");

    assert_eq!(expected_res, actual_res);
}

#[tokio::test]
async fn execute_crash_before() {
    let identity = Module::new_invariant(
        CRASH_BEFORE_EXEC.to_string(),
        "crash_before",
        ValueType::Numeric,
        None,
    )
    .expect("correct inv");

    let geng =
        GengProcess::call_geng(None, 5, &"".to_string(), (None, None)).expect("correct call");
    let reader = geng.get_reader().lines();
    let input = InputFn { reader };
    let error = identity
        .execute(
            &("crash_before".to_string(), 0),
            input,
            NoOutputFn {},
            MAX_STDIN_SIZE,
        )
        .await;

    // let module_exec_error = ModuleExecError {fn_name: "crash_before".to_string(), path: CRASH_BEFORE_EXEC, reason: ExecErrorReason::EarlyExit(_, "")}
    assert!(matches!(
        error,
        Err( ModuleExecError { fn_name, path, reason: ExecErrorReason::EarlyExit(_, _) } )
            if fn_name == "crash_before"
            && std::fs::canonicalize(&path).expect("correct path") == std::fs::canonicalize(CRASH_BEFORE_EXEC).expect("correct path")
    ))
}

#[tokio::test]
async fn execute_crash_during() {
    let identity = Module::new_invariant(
        CRASH_DURING_EXEC.to_string(),
        "crash_during".to_string(),
        ValueType::Numeric,
        None,
    )
    .expect("correct inv");

    let geng =
        GengProcess::call_geng(None, 4, &"".to_string(), (None, None)).expect("correct call");
    let reader = geng.get_reader().lines();
    let input = InputFn { reader };
    let error = identity
        .execute(
            &("crash_during".to_string(), 0),
            input,
            NoOutputFn {},
            MAX_STDIN_SIZE,
        )
        .await;

    assert!(matches!(
        error,
        Err( ModuleExecError { fn_name, path, reason: ExecErrorReason::EarlyExit(_, _) } )
            if fn_name == "crash_during"
            && std::fs::canonicalize(&path).expect("correct path") == std::fs::canonicalize(CRASH_DURING_EXEC).expect("correct path")
    ))
}
