use gquest_core::{
    data_handler::invariant_execs::InvariantsExecutable, utils::config_file::ConfigFile,
};
use serde_json::json;

pub const VALID_EXEC_A: &str = "tests/modules/a.py";
pub const VALID_EXEC_B: &str = "tests/modules/b.py";
pub const VALID_EXEC_C: &str = "tests/modules/c.py";

pub const OPTION_FILE: &str = "tests/modules/dep.json";

#[test]
pub fn from_value_test() {
    // Correct config file to parse
    let config_file = json!({
        "executables": [
            {
                "path": VALID_EXEC_A,
                "names": [
                    "P_Gn"
                ],
                "dep": []
            },
            {
                "path": VALID_EXEC_B,
                "names": [
                    "m",
                    "km",
                    "rm"
                ],
                "dep": []
            },
            {
                "path": VALID_EXEC_C,
                "names": [
                    "is_Bmn"
                ],
                "dep": [
                    "km",
                    "rm"
                ]
            }
        ]
    });

    let (a, b, c) = get_a_b_c_exec();

    let config_file = ConfigFile::read_json_value(config_file).expect("no error");

    assert_eq!([a, b, c].to_vec(), config_file.executables)
}

#[test]
pub fn from_file_test() {
    // Correct config file to parse

    let (a, b, c) = get_a_b_c_exec();

    let config_file = ConfigFile::read_json_file(&OPTION_FILE.to_string()).expect("no error");

    assert_eq!([a, b, c].to_vec(), config_file.executables)
}

pub fn get_a_b_c_exec() -> (
    InvariantsExecutable,
    InvariantsExecutable,
    InvariantsExecutable,
) {
    let a = InvariantsExecutable::new(VALID_EXEC_A.to_string(), vec!["P_Gn".to_string()], vec![])
        .expect("Correct inv");

    let b = InvariantsExecutable::new(
        VALID_EXEC_B.to_string(),
        vec!["m".to_string(), "km".to_string(), "rm".to_string()],
        vec![],
    )
    .expect("Correct inv");

    let c = InvariantsExecutable::new(
        VALID_EXEC_C.to_string(),
        vec!["is_Bmn".to_string()],
        vec!["km".to_string(), "rm".to_string()],
    )
    .expect("Correct inv");

    (a, b, c)
}
