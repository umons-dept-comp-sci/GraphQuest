pub mod config_file;
pub mod csv_utils;
pub mod subject;
pub mod table_handler;

pub trait SaveOutput {
    fn push_line<T: Into<String> + Clone>(&mut self, values: Vec<T>);
}

pub struct StdoutOutput;

impl SaveOutput for StdoutOutput {
    fn push_line<T: Into<String> + Clone>(&mut self, values: Vec<T>) {
        let mut res = String::new();
        if !values.is_empty() {
            for val in values.iter().take(values.len() - 1) {
                res.push_str(&format!("{}, ", val.clone().into()));
            }
            res.push_str(&values.last().expect("present").clone().into().to_string());
        }

        println!("{res}");
    }
}
