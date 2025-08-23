use std::process::{Child, Command, Stdio};

pub struct Invariants {}

impl Invariants {
    pub fn execute() {
        let exec_path_s =
            String::from("/home/axel/GitProject/GraphQuest/core_refactored/examples/size.py");

        let c = exec_command(exec_path_s);
    }
}

fn exec_command(exec_path: String) -> Child {
    Command::new(format!("{}", exec_path))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Could not execute command")
}
