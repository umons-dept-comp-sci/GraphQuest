use gquest_core::{
    data_handler::invariant_execs::Module,
    utils::config_file::{ConfigFile, ConfigFileError},
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
        "modules": [
            {
                "path": VALID_EXEC_A,
                "names": [
                    "P_Gn"
                ],
            },
            {
                "path": VALID_EXEC_B,
                "names": [
                    "m",
                    "km",
                    "rm"
                ],
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

    assert_eq!([a, b, c].to_vec(), config_file.get_execs_ref().clone())
}

#[test]
pub fn from_file_test() {
    // Correct config file to parse
    let (a, b, c) = get_a_b_c_exec();

    let config_file = ConfigFile::read_json_file(&OPTION_FILE.to_string()).expect("no error");

    assert_eq!([a, b, c].to_vec(), config_file.get_execs_ref().clone())
}

pub fn get_a_b_c_exec() -> (Module, Module, Module) {
    let a = Module::new_no_dep(VALID_EXEC_A.to_string(), vec!["P_Gn".to_string()])
        .expect("Correct inv");

    let b = Module::new_no_dep(
        VALID_EXEC_B.to_string(),
        vec!["m".to_string(), "km".to_string(), "rm".to_string()],
    )
    .expect("Correct inv");

    let c = Module::new(
        VALID_EXEC_C.to_string(),
        vec!["is_Bmn".to_string()],
        vec!["km".to_string(), "rm".to_string()],
    )
    .expect("Correct inv");

    (a, b, c)
}

#[test]
pub fn full_config_file_test() {
    // Correct config file to parse
    let config_file = json!({
        "batch_size": 3,
        "nb_threads": 10,
        "modules": [
            {
                "path": VALID_EXEC_A,
                "names": [
                    "P_Gn"
                ],
            }
        ],
        "aliases" : [
            ("is_four_colorable", "chromatic_nb <= 4"),
            ("test", "1 + 1")
        ]
    });
    println!("{config_file}");

    let a = get_a_b_c_exec().0;

    let config_file = ConfigFile::read_json_value(config_file).expect("no error");

    assert_eq!([a].to_vec(), config_file.get_execs_ref().clone());
    assert_eq!(3, config_file.get_batch_size());
    assert_eq!(10, config_file.get_nb_threads());
    assert_eq!(
        &[
            (
                "is_four_colorable".to_string(),
                "chromatic_nb <= 4".to_string()
            ),
            ("test".to_string(), "1 + 1".to_string())
        ]
        .to_vec(),
        config_file.get_aliases()
    );
}

#[test]
pub fn no_batch_size_test() {
    // Correct config file to parse
    let config_file = json!({
        "nb_threads": 10,
        "modules": [
            {
                "path": VALID_EXEC_A,
                "names": [
                    "P_Gn"
                ],
            }
        ]
    });
    ConfigFile::read_json_value(config_file).expect("no error");
}

#[test]
pub fn no_nb_threads_test() {
    // Correct config file to parse
    let config_file = json!({
        "batch_size": 3,
        "modules": [
            {
                "path": VALID_EXEC_A,
                "names": [
                    "P_Gn"
                ],
            }
        ]
    });
    ConfigFile::read_json_value(config_file).expect("no error");
}

#[test]
pub fn empty_modules_test() {
    // Correct config file to parse
    let config_file = json!({
        "batch_size": 3,
        "modules": [
        ]
    });

    assert!(matches!(
        ConfigFile::read_json_value(config_file),
        Err(ConfigFileError::NoInvariantError)
    ))
}
