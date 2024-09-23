use core::fmt;
use std::{fs::File, io::Write, path::Path};


/// Small struct used to make the creation of a *csv* type file easier
/// 
/// # Examples
/// 
/// ```
/// let mut file = CsvFile::new(&String::from("tmp.csv"), Some(';')).expect("Could not create file");
/// let column_names = vec!["column1".to_string(), "column2".to_string()];
/// let values1 = vec![vec!["data 11".to_string(), "data 12".to_string()],
///                    vec!["data 21".to_string(), "data 22".to_string()]];
/// 
/// let values2 = vec![vec!["data 31".to_string(), "data 32".to_string()],
///                    vec!["data 41".to_string(), "data 42".to_string()]];
/// 
/// file.write_lines_to_file(column_names, values1).expect("An error occured while trying to write in the file");
/// file.write_lines_to_file(vec![], values2).expect("An error occured while trying to write in the file");
/// 
/// ```
pub struct CsvFile
{
    /// The file where the data will be stored
    file: File,
    /// The path of the file
    file_path: String,
    /// True if the first line of the csv has not yet been written
    columns_added: bool,
    /// The separator of the data stored in the csv
    separator: char

}

impl CsvFile  {
    /// Creates a new file at the given path (or crushes the one already present) 
    /// # Errors
    /// Will return a [CsvFileError] if there was a problem during the creation of the file
    pub fn new(file_path: &String, separator: Option<char>) -> Result<Self, CsvFileError>
    {
        let path = Path::new(&file_path);
        let file = match File::create(path) {
            Ok(f) => f,
            Err(_) => return Err(CsvFileError::CreationError(file_path.to_string())),
        };
        
        
        Ok(Self {
            file,
            file_path: file_path.to_string(),
            columns_added : true,
            separator: match separator {
                Some(s) => s,
                None => ',',
            }
        })


    }

    /// Writes lines to the csv
    /// 
    /// The `column_names` vector will only be used for the first time this function is called on this struct, after this you can give an empty vec.
    /// # Errors
    /// Will return a [CsvFileError] if there was a problem during the creation of the file
    pub fn write_lines_to_file(&mut self, column_names: Vec<String>, values: Vec<Vec<String>>) -> Result<(), CsvFileError>
    {
        
        // Write the column names
        if self.columns_added {
            self.file.write_all(as_line(&column_names, self.separator).as_bytes()).unwrap();
            self.columns_added = false;
        }
        // Write the values
        for line in values {
            match self.file.write_all(as_line(&line, self.separator).as_bytes()) {
                Ok(_) => (),
                Err(_) => return Err(CsvFileError::WriteError(self.file_path.clone())),
            };
        }

        Ok(())
    }
}



/// Correctly formats the vector as a cvs line, (adds a '\n' to finish the line)
/// # Examples
/// 
/// ```
/// let line: String = as_line(&vec!["data1".to_string(), 
/// "data2".to_string(), 
/// "data3".to_string()], ';');
/// 
/// assert_eq!("data1;data2;data3\n", line);
/// ```

pub fn as_line(values: &Vec<String>, separator: char) -> String
{
    let mut to_write = String::new();
    for value in values {
        to_write.push_str(format!("{}{}", value, separator).as_str())
    }
    to_write.pop(); // remove the last separator
    to_write.push('\n');
    to_write
}



/// Enum used to report a [CsvFile] error
pub enum CsvFileError
{
    /// Is used when there is an error during the creation of the file
    CreationError(String),
    /// Is used when there is an error when trying to write in the file 
    WriteError(String)
}


impl fmt::Display for CsvFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_error_message())
    }
}

impl fmt::Debug for CsvFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreationError(arg0) => f.debug_tuple("CreationError").field(arg0).finish(),
            Self::WriteError(arg0) => f.debug_tuple("WriteError").field(arg0).finish(),
        }
    }
}

impl CsvFileError {
    /// Returns the error message to display for the user
    fn get_error_message(&self) -> String
    {
        match self {
            CsvFileError::CreationError(name) => format!("Could not create file \"{}\"", name),
            CsvFileError::WriteError(name) => format!("Could not write in file \"{}\"", name),
        }
    }
}
