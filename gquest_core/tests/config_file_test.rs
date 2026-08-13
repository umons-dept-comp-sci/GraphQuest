use std::collections::HashMap;

use gquest_core::{
    data_handler::{data_types::ValueType, module::Module},
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
        "batch_size": 6500,
        "modules": [
            {
                "function": "A",
                "path": VALID_EXEC_A,
                "args": [
                    {
                        "name": "n",
                        "class": "bool"
                    }
                ],
                "output": "numeric"
            },
            {
                "function": "B",
                "path": VALID_EXEC_B,
                "output": "string"
            },
            {
                "function": "C",
                "path": VALID_EXEC_C,
                "output": "numeric",
                "batch_size": 100
            }
        ]
    });

    let modules_ref = get_a_b_c_exec();

    let config_file = ConfigFile::read_json_value(config_file).expect("no error");

    let modules = config_file.get_module_refs();

    for (name, module) in modules {
        assert_eq!(module, modules_ref.get(name).expect("fun name present"))
    }
}

#[test]
pub fn from_file_test() {
    // Correct config file to parse
    let module_map = get_a_b_c_exec();

    let config_file = ConfigFile::read_json_file(&OPTION_FILE.to_string()).expect("no error");

    assert_eq!(&module_map, config_file.get_module_refs())
}

pub fn get_a_b_c_exec() -> HashMap<String, Module> {
    let a = Module::new(
        VALID_EXEC_A.to_string(),
        "A",
        [("n", ValueType::Bool)].to_vec(),
        Vec::new(),
        ValueType::Numeric,
        None,
    )
    .expect("Correct inv");

    let b = Module::new_invariant(VALID_EXEC_B.to_string(), "B", ValueType::String, None)
        .expect("Correct inv");

    let c = Module::new_invariant(VALID_EXEC_C.to_string(), "C", ValueType::Numeric, Some(100))
        .expect("Correct inv");
    let mut hash_map = HashMap::new();

    hash_map.insert("A".to_string(), a);
    hash_map.insert("B".to_string(), b);
    hash_map.insert("C".to_string(), c);

    hash_map
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
