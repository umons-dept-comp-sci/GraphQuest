use std::io::{BufRead, Read};

use gquest_core::data_handler::{
    data_loader::GengProcess,
    invariant_execs::{InvariantExecutionError, InvariantsExecutable},
};

const MAX_STDIN_SIZE: usize = 40;

pub const VALID_EXEC_IDENTITY: &str = "tests/modules/identity.py";
pub const LATE_FLUSH_EXEC: &str = "tests/modules/late_flush.py";
pub const CRASH_BEFORE_EXEC: &str = "tests/modules/crash_before.py";
pub const CRASH_DURING_EXEC: &str = "tests/modules/crash_during.py";

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
    let mut reader = geng.get_reader().lines();
    let mut actual_res: Vec<String> = vec![];
    identity
        .execute_invariant(
            &mut async || -> Option<Result<Vec<String>, ()>> {
                let v = reader.next()?.ok()?;
                Some(Ok([v].to_vec()))
            },
            &mut async |s| {
                actual_res.push(s[1].clone());
                Ok(())
            },
            MAX_STDIN_SIZE,
        )
        .await
        .expect("ok");

    assert_eq!(expected_res, actual_res);
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
    let mut reader = geng.get_reader().lines();
    let mut actual_res: Vec<String> = vec![];
    identity
        .execute_invariant(
            &mut async || -> Option<Result<Vec<String>, ()>> {
                let v = reader.next()?.ok()?;
                Some(Ok([v].to_vec()))
            },
            &mut async |s| {
                actual_res.push(s[1].clone());
                Ok(())
            },
            MAX_STDIN_SIZE,
        )
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
    let mut reader = geng.get_reader().lines();
    let error = identity
        .execute_invariant(
            &mut async || -> Option<Result<Vec<String>, ()>> {
                let v = reader.next()?.ok()?;
                Some(Ok([v].to_vec()))
            },
            &mut async |_| Ok(()),
            MAX_STDIN_SIZE,
        )
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
    let mut reader = geng.get_reader().lines();
    let error = identity
        .execute_invariant(
            &mut async || -> Option<Result<Vec<String>, ()>> {
                let v = reader.next()?.ok()?;
                Some(Ok([v].to_vec()))
            },
            &mut async |_| Ok(()),
            MAX_STDIN_SIZE,
        )
        .await;

    assert!(matches!(
        error,
        Err(InvariantExecutionError::EarlyExit(_, _, _))
    ))
}


// TODO: Tests -> unexpected input and outputs

// Also the test when the input function fails during exec