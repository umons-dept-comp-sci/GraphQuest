use std::{
    io::{BufRead, BufReader, Lines, Read},
    process::ChildStdout,
};

use gquest_core::data_handler::{
    data_loader::GengProcess,
    invariant_execs::{
        AsyncInvariantInput, AsyncInvariantOutput, InvariantExecutionError, InvariantsExecutable,
    },
};

const MAX_STDIN_SIZE: usize = 40;

pub const VALID_EXEC_IDENTITY: &str = "tests/modules/identity.py";
pub const LATE_FLUSH_EXEC: &str = "tests/modules/late_flush.py";
pub const CRASH_BEFORE_EXEC: &str = "tests/modules/crash_before.py";
pub const CRASH_DURING_EXEC: &str = "tests/modules/crash_during.py";

struct InputFn {
    reader: Lines<BufReader<ChildStdout>>,
}

impl AsyncInvariantInput<()> for InputFn {
    async fn call(&mut self) -> Option<Result<Vec<String>, ()>> {
        let v = self.reader.next()?.ok()?;
        Some(Ok([v].to_vec()))
    }
}

struct OutputFn<'e> {
    output_buffer: &'e mut Vec<String>,
}

impl<'e> AsyncInvariantOutput<()> for OutputFn<'e> {
    async fn call(&mut self, values: Vec<String>) -> Result<(), ()> {
        self.output_buffer.push(values[1].clone());
        Ok(())
    }
}

struct NoOutputFn {}

impl AsyncInvariantOutput<()> for NoOutputFn {
    async fn call(&mut self, _values: Vec<String>) -> Result<(), ()> {
        Ok(())
    }
}

#[tokio::test]
async fn execute_correct_inv() {
    let identity = InvariantsExecutable::new_no_dep(
        VALID_EXEC_IDENTITY.to_string(),
        ['x'.to_string()].to_vec(),
    )
    .expect("correct inv");

    let geng = GengProcess::call_geng(5, &"".to_string(), (None, None)).expect("correct call");
    let mut res: String = String::default();
    geng.get_reader().read_to_string(&mut res).expect("correct");
    let expected_res: Vec<&str> = res.split_ascii_whitespace().collect();

    let geng = GengProcess::call_geng(5, &"".to_string(), (None, None)).expect("correct call");
    let reader = geng.get_reader().lines();
    let input = InputFn { reader };

    let mut output_buffer = vec![];

    let output = OutputFn {
        output_buffer: &mut output_buffer,
    };

    identity
        .execute_invariant(input, output, MAX_STDIN_SIZE)
        .await
        .expect("ok");

    assert_eq!(expected_res, output_buffer);
}

#[tokio::test]
async fn execute_late_inv() {
    let identity = InvariantsExecutable::new_no_dep(LATE_FLUSH_EXEC.to_string(), ['x'].to_vec())
        .expect("correct inv");

    let geng = GengProcess::call_geng(5, &"".to_string(), (None, None)).expect("correct call");
    let mut res: String = String::default();
    geng.get_reader().read_to_string(&mut res).expect("correct");
    let expected_res: Vec<&str> = res.split_ascii_whitespace().collect();

    let geng = GengProcess::call_geng(5, &"".to_string(), (None, None)).expect("correct call");
    let reader = geng.get_reader().lines();
    let input = InputFn { reader };
    let mut actual_res: Vec<String> = vec![];

    let output = OutputFn {
        output_buffer: &mut actual_res,
    };

    identity
        .execute_invariant(input, output, MAX_STDIN_SIZE)
        .await
        .expect("ok");

    assert_eq!(expected_res, actual_res);
}

#[tokio::test]
async fn execute_crash_before() {
    let identity =
        InvariantsExecutable::new_no_dep(CRASH_BEFORE_EXEC.to_string(), ['x'.to_string()].to_vec())
            .expect("correct inv");

    let geng = GengProcess::call_geng(5, &"".to_string(), (None, None)).expect("correct call");
    let reader = geng.get_reader().lines();
    let input = InputFn { reader };
    let error = identity
        .execute_invariant(input, NoOutputFn {}, MAX_STDIN_SIZE)
        .await;

    assert!(matches!(
        error,
        Err(InvariantExecutionError::EarlyExit(_, _, _))
    ))
}

#[tokio::test]
async fn execute_crash_during() {
    let identity =
        InvariantsExecutable::new_no_dep(CRASH_DURING_EXEC.to_string(), ['x'.to_string()].to_vec())
            .expect("correct inv");

    let geng = GengProcess::call_geng(4, &"".to_string(), (None, None)).expect("correct call");
    let reader = geng.get_reader().lines();
    let input = InputFn { reader };
    let error = identity
        .execute_invariant(input, NoOutputFn {}, MAX_STDIN_SIZE)
        .await;

    assert!(matches!(
        error,
        Err(InvariantExecutionError::EarlyExit(_, _, _))
    ))
}

// TODO: Tests -> unexpected input and outputs

// Also the test when the input function fails during exec
