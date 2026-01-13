use gquest_core::data_handler::invariant_execs::{Module, ModuleError, ModuleSorter};

pub const VALID_EXEC_A: &str = "tests/modules/a.py";
pub const VALID_EXEC_B: &str = "tests/modules/b.py";
pub const VALID_EXEC_C: &str = "tests/modules/c.py";
pub const VALID_EXEC_IDENTITY: &str = "tests/modules/identity.py";

pub const UNVALID_EXEC: &str = "tests/modules/non_executable.py";

#[test]
fn new_inv_exec_path_test() {
    let _ = Module::new_no_dep(VALID_EXEC_A.to_string(), vec!["size".to_string()])
        .expect("Correct path");

    if Module::new_no_dep("WRONG_PATH.py".to_string(), vec!["size".to_string()]).is_ok() {
        panic!("Should return Err")
    }
}

#[test]
fn new_non_exec_inv_exec_test() {
    if Module::new_no_dep(UNVALID_EXEC.to_string(), vec!["size".to_string()]).is_ok() {
        panic!("Should return Err")
    }
}

#[test]
fn new_inv_exec_name_test() {
    let _ = Module::new_no_dep(VALID_EXEC_A.to_string(), vec!["size".to_string()])
        .expect("Correct name");

    if Module::new_no_dep(VALID_EXEC_A.to_string(), vec!["3size".to_string()]).is_ok() {
        panic!("Should return Err");
    }

    if Module::new_no_dep(VALID_EXEC_A.to_string(), vec!["🫡fail".to_string()]).is_ok() {
        panic!("Should return Err");
    }

    if Module::new_no_dep(VALID_EXEC_A.to_string(), vec!["fail space".to_string()]).is_ok() {
        panic!("Should return Err");
    }
}

#[test]
fn new_inv_exec_rely_self_test() {
    if Module::new(
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

    let mut order = ModuleSorter::new();
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
    let a = Module::new(
        VALID_EXEC_A.to_string(),
        vec!["a".to_string()],
        vec!["b".to_string(), "c".to_string()],
    )
    .expect("Correct inv");

    let b = Module::new(
        VALID_EXEC_B.to_string(),
        vec!["b".to_string(), "a".to_string()],
        vec!["c".to_string()],
    )
    .expect("Correct inv");

    let mut order = ModuleSorter::new();
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
    let a = Module::new(
        VALID_EXEC_A.to_string(),
        vec!["a".to_string()],
        vec!["b".to_string(), "c".to_string()],
    )
    .expect("Correct inv");

    let b = Module::new(
        VALID_EXEC_B.to_string(),
        vec!["b".to_string()],
        vec!["c".to_string()],
    )
    .expect("Correct inv");

    let c = Module::new(
        VALID_EXEC_C.to_string(),
        vec!["c".to_string()],
        vec!["a".to_string()],
    )
    .expect("Correct inv");

    let mut order = ModuleSorter::new();
    order.add_inv_exec(a.clone()).expect("no issues");
    order.add_inv_exec(b.clone()).expect("no issues");
    order.add_inv_exec(c.clone()).expect("no issues");

    if order.sort().is_ok() {
        panic!("Should be cycle error")
    }
}

#[test]
fn new_dep_missing_inv_order_test() {
    let a = Module::new(
        VALID_EXEC_A.to_string(),
        vec!["a".to_string()],
        vec!["b".to_string(), "c".to_string()],
    )
    .expect("Correct inv");

    let b = Module::new(
        VALID_EXEC_B.to_string(),
        vec!["b".to_string()],
        vec!["d".to_string()],
    )
    .expect("Correct inv");

    let c =
        Module::new_no_dep(VALID_EXEC_C.to_string(), vec!["c".to_string()]).expect("Correct inv");

    let mut order = ModuleSorter::new();
    order.add_inv_exec(a.clone()).expect("no issues");
    order.add_inv_exec(b.clone()).expect("no issues");
    order.add_inv_exec(c.clone()).expect("no issues");

    assert!(matches!(
        order.sort(),
        Err(ModuleError::MissingDependency(_, _))
    ));
}

#[test]
fn new_exec_iterator_test() {
    let (a, b, c) = get_a_b_c_exec();

    let order: ModuleSorter = [a.clone(), b.clone(), c.clone()]
        .to_vec()
        .try_into()
        .expect("ok");

    let man = order.to_iter().expect("no error");

    let execs_sorted: Vec<Module> = man.into_iter().collect();

    assert!(execs_sorted[0].eq(&c));
    assert!(execs_sorted[1].eq(&b));
    assert!(execs_sorted[2].eq(&a));
}

#[test]
fn new_exec_group_test_no_dep() {
    let a = Module::new_no_dep(VALID_EXEC_A, vec!["a"]).expect("Correct inv");
    let b = Module::new_no_dep(VALID_EXEC_B, vec!["b"]).expect("Correct inv");
    let c = Module::new_no_dep(VALID_EXEC_C, vec!["c", "d"]).expect("Correct inv");

    let order: ModuleSorter = [a.clone(), c.clone(), b.clone()]
        .to_vec()
        .try_into()
        .expect("ok");

    let man = order.to_iter().expect("no error");

    let execs_sorted: Vec<Module> = man.into_iter().collect();
    assert_eq!(3, execs_sorted.len());
    assert!(execs_sorted.contains(&c));
    assert!(execs_sorted.contains(&b));
    assert!(execs_sorted.contains(&a));
}

pub fn get_a_b_c_exec() -> (Module, Module, Module) {
    let a = Module::new(VALID_EXEC_A, vec!["a"], vec!["b", "c"]).expect("Correct inv");

    let b = Module::new(VALID_EXEC_B, vec!["b"], vec!["c"]).expect("Correct inv");

    let c = Module::new_no_dep(VALID_EXEC_C, vec!["c"]).expect("Correct inv");

    (a, b, c)
}
