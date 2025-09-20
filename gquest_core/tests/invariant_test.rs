use gquest_core::data_handler::invariants::{ExecutableOrderHandler, InvariantsExecutable};

pub const VALID_EXEC_A: &str = "examples/a.py";
pub const VALID_EXEC_B: &str = "examples/b.py";
pub const VALID_EXEC_C: &str = "examples/c.py";

#[test]
fn new_inv_exec_path_test() {
    let _ = InvariantsExecutable::new(VALID_EXEC_A.to_string(), vec!["size".to_string()], vec![])
        .expect("Correct path");

    if InvariantsExecutable::new(
        "WRONG_PATH.py".to_string(),
        vec!["size".to_string()],
        vec![],
    )
    .is_ok()
    {
        panic!("Should return Err")
    }
}

#[test]
fn new_inv_exec_name_test() {
    let _ = InvariantsExecutable::new(VALID_EXEC_A.to_string(), vec!["size".to_string()], vec![])
        .expect("Correct name");

    if InvariantsExecutable::new(VALID_EXEC_A.to_string(), vec!["3size".to_string()], vec![])
        .is_ok()
    {
        panic!("Should return Err");
    }

    if InvariantsExecutable::new(VALID_EXEC_A.to_string(), vec!["🫡fail".to_string()], vec![])
        .is_ok()
    {
        panic!("Should return Err");
    }

    if InvariantsExecutable::new(
        VALID_EXEC_A.to_string(),
        vec!["fail space".to_string()],
        vec![],
    )
    .is_ok()
    {
        panic!("Should return Err");
    }
}

#[test]
fn new_inv_exec_rely_self_test() {
    if InvariantsExecutable::new(
        VALID_EXEC_A.to_string(),
        vec!["invariant".to_string()],
        vec!["one".to_string(), "invariant".to_string()],
    )
    .is_ok()
    {
        panic!("Should return Err");
    }
}

#[test]
fn new_correct_inv_order_test() {
    let a = InvariantsExecutable::new(
        VALID_EXEC_A.to_string(),
        vec!["a".to_string()],
        vec!["b".to_string(), "c".to_string()],
    )
    .expect("Correct inv");

    let b = InvariantsExecutable::new(
        VALID_EXEC_B.to_string(),
        vec!["b".to_string()],
        vec!["c".to_string()],
    )
    .expect("Correct inv");

    let c = InvariantsExecutable::new(VALID_EXEC_C.to_string(), vec!["c".to_string()], vec![])
        .expect("Correct inv");

    let mut order = ExecutableOrderHandler::new();
    order.add_inv_exec(a.clone()).expect("no issues");
    order.add_inv_exec(b.clone()).expect("no issues");
    order.add_inv_exec(c.clone()).expect("no issues");

    let ordered_executables = order.sort().expect("no issues");
    assert_eq!(c, ordered_executables[0]);
    assert_eq!(b, ordered_executables[1]);
    assert_eq!(a, ordered_executables[2]);
}
