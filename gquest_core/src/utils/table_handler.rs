use std::{collections::VecDeque, fmt::Display, vec};
use tabled::{builder::Builder, settings::Style};

/// Enum used to specify the options to use when creating a [QueryTable]
#[derive(Debug, Clone)]
pub enum QueryTableOptions {
    /// The [QueryTable] will store and display all values
    Full,
    /// The [QueryTable] will only store the first and last specified added rows, use this when there is too much data to display
    Partial {
        /// The number of first rows to display
        first_rows_count: usize,
        /// The number of last rows to display
        last_rows_count: usize,
    },
}

impl QueryTableOptions {
    fn add_line(&self, data: &mut TableData, row: Vec<String>) {
        match self {
            QueryTableOptions::Full => {
                data.first_lines.push(row);
            }
            QueryTableOptions::Partial {
                first_rows_count,
                last_rows_count,
            } => {
                if data.first_lines.len() <= *first_rows_count {
                    data.first_lines.push(row);
                } else {
                    if data.last_lines.len() == *last_rows_count {
                        data.has_overflown = true;
                        // Forget oldest line
                        data.last_lines.pop_front();
                    }
                    if *last_rows_count != 0 {
                        data.last_lines.push_back(row);
                    }
                }
            }
        }
    }

    fn build(&self, data: &TableData, builder: &mut Builder) {
        for line in &data.first_lines {
            builder.push_record(line.clone());
        }
        if let QueryTableOptions::Partial {
            first_rows_count: _,
            last_rows_count: _,
        } = self
        {
            if data.has_overflown {
                builder.push_record((0..builder.count_columns()).map(|_| String::from("...")));
            }

            for line in &data.last_lines {
                builder.push_record(line.clone());
            }
        }
    }
}

#[derive(Default)]
/// Used to store data when creating a partial query table
struct TableData {
    /// Checks if the max first rows was reached so that we can display `...` in the table  
    has_overflown: bool,
    /// The first lines to store in the table
    first_lines: Vec<Vec<String>>,
    /// The last lines to store in the table
    last_lines: VecDeque<Vec<String>>,
}

/// Struct representing a stylized table that can be used to store and display informations in a clean way
pub struct QueryTable {
    curr_index: usize,
    table_data: TableData,
    mode: QueryTableOptions,
}

impl QueryTable {
    /// Creates a new empty [`QueryTable`]
    pub fn new<T: Into<String> + Clone>(header: Vec<T>, mode: QueryTableOptions) -> Self {
        let mut res = Self {
            curr_index: 0,
            table_data: TableData::default(),
            mode,
        };

        let mut header_clone = vec![String::from("i")];
        header_clone.append(&mut to_vec_string(header));

        res.mode.add_line(&mut res.table_data, header_clone);

        res
    }

    /// Push the given lines in the table, while respecting the options of the table
    pub fn push_line<T: Into<String> + Clone>(&mut self, values: Vec<T>) {
        let mut values_indexed = vec![self.curr_index.to_string()];
        values_indexed.append(&mut to_vec_string(values));

        self.mode.add_line(&mut self.table_data, values_indexed);
        self.curr_index += 1;
    }
}

fn to_vec_string<T: Into<String> + Clone>(vec: Vec<T>) -> Vec<String> {
    let mut res = Vec::new();

    for value in vec {
        res.push(value.into());
    }

    res
}

impl Display for QueryTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", {
            let mut builder = Builder::default();
            if self.table_data.first_lines.is_empty() && self.table_data.last_lines.is_empty() {
                builder.push_record(vec!["     /     "]);
            }

            self.mode.build(&self.table_data, &mut builder);

            let mut t = builder.build();
            t.with(Style::rounded());
            t.to_string()
        })
    }
}
