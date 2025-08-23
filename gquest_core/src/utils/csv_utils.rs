use std::{fmt::Display, fs::File, io::Write, path::Path};

/// Small struct used to make the creation of a *csv* type file easier
pub struct CsvFile {
    /// The file where the data will be stored
    file: File,
    /// The path of the file
    file_path: String,
    /// The separator of the data stored in the csv
    separator: char,
}
//______________________________ CSV FILE FUNCTIONS
impl CsvFile {
    /// Creates a new file at the given path (or crushes the one already present)
    /// # Errors
    /// Will return a [CsvFileError] if there was a problem during the creation of the file
    pub fn new<T: Into<String> + Clone>(
        file_path: &String,
        separator: Option<char>,
        column_names: Vec<T>,
    ) -> Result<Self, CsvFileError> {
        let path = Path::new(&file_path);
        let file = match File::create(path) {
            Ok(f) => f,
            Err(_) => return Err(CsvFileError::CreationError(file_path.to_string())),
        };
        let mut res = Self {
            file,
            file_path: file_path.to_string(),
            separator: separator.unwrap_or(','),
        };

        // Write the column names
        res.file
            .write_all(as_line(&column_names, res.separator).as_bytes())
            .unwrap();

        Ok(res)
    }

    /// Writes lines to the csv
    ///
    /// The `column_names` vector will only be used for the first time this function is called on this struct, after this you can give an empty vec.
    /// # Errors
    /// Will return a [CsvFileError] if there was a problem during the creation of the file
    pub fn write_lines_to_file<T: Into<String> + Clone>(
        &mut self,
        values: Vec<Vec<T>>,
    ) -> Result<(), CsvFileError> {

        // Write the values
        for line in values {
            match self
                .file
                .write_all(as_line(&line, self.separator).as_bytes())
            {
                Ok(_) => (),
                Err(_) => return Err(CsvFileError::WriteError(self.file_path.clone())),
            };
        }

        Ok(())
    }
}

/// Correctly formats the vector as a cvs line, (adds a '\n' to finish the line)
pub fn as_line<T: Into<String> + Clone>(values: &Vec<T>, separator: char) -> String {
    let mut to_write = String::new();
    for value in values {
        let value : String = value.clone().into();
        to_write.push_str(format!("{}{}", value, separator).as_str())
    }
    to_write.pop(); // remove the last separator
    to_write.push('\n');
    to_write
}

//______________________________ ERRORS STRUCT

#[derive(Debug)]
/// Enum used to report a [CsvFile] error
pub enum CsvFileError {
    /// Is used when there is an error during the creation of the file
    CreationError(String),
    /// Is used when there is an error when trying to write in the file
    WriteError(String),
}

impl Display for CsvFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_error_message())
    }
}

impl CsvFileError {
    /// Returns the error message to display for the user
    fn get_error_message(&self) -> String {
        match self {
            CsvFileError::CreationError(name) => format!("Could not create file \"{}\"", name),
            CsvFileError::WriteError(name) => format!("Could not write in file \"{}\"", name),
        }
    }
}
