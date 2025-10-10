use std::io::Read;

use gquest_core::data_handler::{
    data_loader::GengProcess,
    invariant_execs::{InvariantExecutionError, InvariantsExecutable},
};

pub const VALID_EXEC_IDENTITY: &str = "tests/modules/identity.py";
pub const CRASH_BEFORE_EXEC: &str = "tests/modules/crash_before.py";
pub const CRASH_DURING_EXEC: &str = "tests/modules/crash_during.py";

#[tokio::test]
async fn execute_correct_inv() {
    let identity = InvariantsExecutable::new(
        VALID_EXEC_IDENTITY.to_string(),
        ['x'.to_string()].to_vec(),
        [].to_vec(),
    )
    .expect("correct inv");

    let geng = GengProcess::call_geng(5, &"".to_string(), (None, None)).expect("correct call");
    let mut res: String = String::default();
    geng.get_reader().read_to_string(&mut res).expect("correct");
    let expected_res: Vec<&str> = res.split_ascii_whitespace().collect();

    let geng = GengProcess::call_geng(5, &"".to_string(), (None, None)).expect("correct call");
    let mut actual_res: Vec<String> = vec![];
    identity
        .execute_invariant(geng.get_reader(), &mut async |s| {
            actual_res.push(s);
        })
        .await
        .expect("ok");

    assert_eq!(expected_res, actual_res);
}

#[tokio::test]
async fn execute_inv_before() {
    let identity = InvariantsExecutable::new(
        CRASH_BEFORE_EXEC.to_string(),
        ['x'.to_string()].to_vec(),
        [].to_vec(),
    )
    .expect("correct inv");

    let geng = GengProcess::call_geng(5, &"".to_string(), (None, None)).expect("correct call");

    let error = identity
        .execute_invariant(geng.get_reader(), &mut async |_| {})
        .await;

    assert!(matches!(
        error,
        Err(InvariantExecutionError::EarlyExit(_, _, _))
    ))
}

#[tokio::test]
async fn execute_inv_during() {
    let identity = InvariantsExecutable::new(
        CRASH_DURING_EXEC.to_string(),
        ['x'.to_string()].to_vec(),
        [].to_vec(),
    )
    .expect("correct inv");

    let geng = GengProcess::call_geng(5, &"".to_string(), (None, None)).expect("correct call");

    let error = identity
        .execute_invariant(geng.get_reader(), &mut async |_| {})
        .await;

    assert!(matches!(
        error,
        Err(InvariantExecutionError::EarlyExit(_, _, _))
    ))
}
