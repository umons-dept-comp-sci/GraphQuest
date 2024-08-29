use std::collections::VecDeque;

use tabled::{builder::Builder, settings::Style};


/// Enum used to specify the options to use when creating a [QueryTable]
#[derive(Debug)]
pub enum QueryTableOptions
{
    /// The [QueryTable] will store and display all values
    Full,
    /// The [QueryTable] will only store the first and last specified added rows, use this when there is too much data to display 
    Partial
    {
        /// The number of first rows to display
        first_rows_count:usize,
        /// The number of last rows to display 
        last_rows_count: usize
    },
}

/// Used to store data when creating a partial query table
struct TableData
{
    max_first_last_rows_count: (usize, usize),
    has_overflown: bool,
    first_lines: Vec<Vec<String>>,
    last_lines: VecDeque<Vec<String>>,
}

impl TableData {
    fn add_line(&mut self, value: Vec<String>)
    {
        let (first_count, last_count) = self.max_first_last_rows_count;

        // If the first lines weren't filled
        if self.first_lines.len() < first_count{
            self.first_lines.push(value);
        }
        // new last line
        else {
            if self.last_lines.len() == last_count {
                self.has_overflown = true;
                // forget the oldest line
                self.last_lines.pop_front();
            }
            if last_count != 0{
                
                self.last_lines.push_back(value);
            }
        }
    }

    fn build(self, builder: &mut Builder)
    {
        // Add first lines
        let mut line_length = 0;
        for line in self.first_lines {
            line_length = line.len();
            builder.push_record(line);
        }
        if self.has_overflown {
            if  self.last_lines.len() != 0{
                line_length = self.last_lines[0].len();
            }
            builder.push_record((0..line_length).map(|_| String::from("...")));
        }

        // Add last lines
        for line in self.last_lines {
            builder.push_record(line);
        }
    }
}


pub struct QueryTable
{
    builder: Builder,
    curr_index: usize,
    headers_added : bool,
    table_data: Option<TableData>,
}


impl QueryTable {
    // Creates a new empty [QueryTable]
    pub fn new(options: QueryTableOptions) -> Self
    {
        let mut builder = Builder::default();
        
        let table_data: Option<TableData> = {
            match options{
                QueryTableOptions::Full => None,
                QueryTableOptions::Partial { first_rows_count, last_rows_count } => 
                    Some(TableData {
                        has_overflown: false,
                        first_lines: Vec::new(),
                        last_lines: VecDeque::new(),
                        max_first_last_rows_count: (first_rows_count, last_rows_count)
                    }),
            }
        };
        Self {
            builder,
            curr_index: 0,
            headers_added: false,
            table_data
        }
    }


    pub fn set_headers(&mut self, mut columns: Vec<String>)
    {
        let mut col: Vec<String> = vec![String::from("i")];
        self.headers_added = true;
        col.append(&mut columns);
        self.builder.insert_record(0, col);
    }

    pub fn headers_added(&self) -> bool
    {
        return self.headers_added;
    }


    pub fn push_line(&mut self, mut values: Vec<String>)
    {
        let mut line: Vec<String> = vec![self.curr_index.to_string()];
        line.append(&mut values);

        if let Some(opt) = &mut self.table_data {
            opt.add_line(line);
        }
        // Simply add the values
        else{
            self.builder.push_record(line);
        }
        self.curr_index += 1;
    }


    pub fn as_string(mut self) -> String
    {
        if let Some(opt) = self.table_data {
            opt.build(&mut self.builder);
        }
        // If the table is empty with no headers
        if self.curr_index == 0 && !self.headers_added {
            
            self.builder.push_record(vec!["EMPTY TABLE"]);
        }
        if self.curr_index == 0 {
            
            self.builder.push_record(vec!["     /     "]);
        }
        
        let mut t = self.builder.build();
        t.with(Style::rounded());
        t.to_string()
    }
}