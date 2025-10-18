use gquest_core::data_handler::invariant_execs::{
    ExecutableSorter, InvariantError, InvariantsExecutable,
};

pub const VALID_EXEC_A: &str = "tests/modules/a.py";
pub const VALID_EXEC_B: &str = "tests/modules/b.py";
pub const VALID_EXEC_C: &str = "tests/modules/c.py";
pub const VALID_EXEC_IDENTITY: &str = "tests/modules/identity.py";

pub const UNVALID_EXEC: &str = "tests/modules/non_executable.py";

#[test]
fn new_inv_exec_path_test() {
    let _ = InvariantsExecutable::new_no_dep(VALID_EXEC_A.to_string(), vec!["size".to_string()])
        .expect("Correct path");

    if InvariantsExecutable::new_no_dep("WRONG_PATH.py".to_string(), vec!["size".to_string()])
        .is_ok()
    {
        panic!("Should return Err")
    }
}

#[test]
fn new_non_exec_inv_exec_test() {
    if InvariantsExecutable::new_no_dep(UNVALID_EXEC.to_string(), vec!["size".to_string()]).is_ok()
    {
        panic!("Should return Err")
    }
}

#[test]
fn new_inv_exec_name_test() {
    let _ = InvariantsExecutable::new_no_dep(VALID_EXEC_A.to_string(), vec!["size".to_string()])
        .expect("Correct name");

    if InvariantsExecutable::new_no_dep(VALID_EXEC_A.to_string(), vec!["3size".to_string()]).is_ok()
    {
        panic!("Should return Err");
    }

    if InvariantsExecutable::new_no_dep(VALID_EXEC_A.to_string(), vec!["🫡fail".to_string()])
        .is_ok()
    {
        panic!("Should return Err");
    }

    if InvariantsExecutable::new_no_dep(VALID_EXEC_A.to_string(), vec!["fail space".to_string()])
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
    let (a, b, c) = get_a_b_c_exec();

    let mut order = ExecutableSorter::new();
    order.add_inv_exec(a.clone()).expect("no issues");
    order.add_inv_exec(b.clone()).expect("no issues");
    order.add_inv_exec(c.clone()).expect("no issues");

    let ordered_executables = order.sort().expect("no issues");
    assert_eq!(c, ordered_executables[0]);
    assert_eq!(b, ordered_executables[1]);
    assert_eq!(a, ordered_executables[2]);
}

#[test]
fn new_incorrect_inv_test() {
    let a = InvariantsExecutable::new(
        VALID_EXEC_A.to_string(),
        vec!["a".to_string()],
        vec!["b".to_string(), "c".to_string()],
    )
    .expect("Correct inv");

    let b = InvariantsExecutable::new(
        VALID_EXEC_B.to_string(),
        vec!["b".to_string(), "a".to_string()],
        vec!["c".to_string()],
    )
    .expect("Correct inv");

    let mut order = ExecutableSorter::new();
    order.add_inv_exec(a.clone()).expect("no issues");
    if order.add_inv_exec(a).is_ok() {
        panic!("should be an error");
    }

    if order.add_inv_exec(b).is_ok() {
        panic!("should be an error");
    }
}

#[test]
fn new_cycle_inv_order_test() {
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

    let c = InvariantsExecutable::new(
        VALID_EXEC_C.to_string(),
        vec!["c".to_string()],
        vec!["a".to_string()],
    )
    .expect("Correct inv");

    let mut order = ExecutableSorter::new();
    order.add_inv_exec(a.clone()).expect("no issues");
    order.add_inv_exec(b.clone()).expect("no issues");
    order.add_inv_exec(c.clone()).expect("no issues");

    if order.sort().is_ok() {
        panic!("Should be cycle error")
    }
}

#[test]
fn new_dep_missing_inv_order_test() {
    let a = InvariantsExecutable::new(
        VALID_EXEC_A.to_string(),
        vec!["a".to_string()],
        vec!["b".to_string(), "c".to_string()],
    )
    .expect("Correct inv");

    let b = InvariantsExecutable::new(
        VALID_EXEC_B.to_string(),
        vec!["b".to_string()],
        vec!["d".to_string()],
    )
    .expect("Correct inv");

    let c = InvariantsExecutable::new_no_dep(VALID_EXEC_C.to_string(), vec!["c".to_string()])
        .expect("Correct inv");

    let mut order = ExecutableSorter::new();
    order.add_inv_exec(a.clone()).expect("no issues");
    order.add_inv_exec(b.clone()).expect("no issues");
    order.add_inv_exec(c.clone()).expect("no issues");

    assert!(matches!(
        order.sort(),
        Err(InvariantError::MissingDependency(_, _))
    ));
}

#[test]
fn new_exec_group_test_all_sep() {
    let (a, b, c) = get_a_b_c_exec();

    let order: ExecutableSorter = [a.clone(), b.clone(), c.clone()]
        .to_vec()
        .try_into()
        .expect("ok");

    let man = order.group_execs().expect("no error");

    assert!(man.get_groups()[0].contains(&c));
    assert!(man.get_groups()[1].contains(&b));
    assert!(man.get_groups()[2].contains(&a));
}

#[test]
fn new_exec_group_test_same_dep() {
    let a =
        InvariantsExecutable::new(VALID_EXEC_A, vec!["a"], vec!["d", "c"]).expect("Correct inv");
    let b = InvariantsExecutable::new(VALID_EXEC_B, vec!["b"], vec!["c"]).expect("Correct inv");
    let c = InvariantsExecutable::new_no_dep(VALID_EXEC_C, vec!["c", "d"]).expect("Correct inv");

    let order: ExecutableSorter = [a.clone(), c.clone(), b.clone()]
        .to_vec()
        .try_into()
        .expect("ok");

    let man = order.group_execs().expect("no error");

    assert!(man.get_groups()[0].contains(&c));
    assert!(man.get_groups()[1].contains(&b));
    assert!(man.get_groups()[1].contains(&a));
}

#[test]
fn new_exec_group_test_no_dep() {
    let a = InvariantsExecutable::new_no_dep(VALID_EXEC_A, vec!["a"]).expect("Correct inv");
    let b = InvariantsExecutable::new_no_dep(VALID_EXEC_B, vec!["b"]).expect("Correct inv");
    let c = InvariantsExecutable::new_no_dep(VALID_EXEC_C, vec!["c", "d"]).expect("Correct inv");

    let order: ExecutableSorter = [a.clone(), c.clone(), b.clone()]
        .to_vec()
        .try_into()
        .expect("ok");

    let man = order.group_execs().expect("no error");

    assert!(man.get_groups()[0].contains(&c));
    assert!(man.get_groups()[0].contains(&b));
    assert!(man.get_groups()[0].contains(&a));
}

pub fn get_a_b_c_exec() -> (
    InvariantsExecutable,
    InvariantsExecutable,
    InvariantsExecutable,
) {
    let a =
        InvariantsExecutable::new(VALID_EXEC_A, vec!["a"], vec!["b", "c"]).expect("Correct inv");

    let b = InvariantsExecutable::new(VALID_EXEC_B, vec!["b"], vec!["c"]).expect("Correct inv");

    let c = InvariantsExecutable::new_no_dep(VALID_EXEC_C, vec!["c"]).expect("Correct inv");

    (a, b, c)
}
