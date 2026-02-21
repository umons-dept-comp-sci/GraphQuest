use std::{collections::VecDeque, fmt::Display, vec};
use tabled::{Table, builder::Builder, settings::Style};

use crate::utils::SaveOutput;

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
    header_added: bool,
    mode: QueryTableOptions,
}

impl QueryTable {
    /// Creates a new [`QueryTable`] with the given header.
    pub fn new<T: Into<String> + Clone>(header: Vec<T>, mode: QueryTableOptions) -> Self {
        let mut res = Self::new_no_header(mode);

        res.push_line(header);

        res
    }

    /// Creates a new empty [`QueryTable`] where the first added line will be considered as the header.
    pub fn new_no_header(mode: QueryTableOptions) -> Self {
        Self {
            curr_index: 0,
            table_data: TableData::default(),
            header_added: false,
            mode,
        }
    }

    /// Turns the current table into a valid latex table
    /// (also sanitizes the data to prevent any problems).
    pub fn to_latex(&self) -> String {
        if !self.header_added {
            // Return Empty table
            return String::from(
                r"\begin{table}[]
\begin{tabular}{|l|}
\hline
\textbf{Empty table}    \\ \hline
\multicolumn{1}{|c|}{/} \\ \hline
\end{tabular}
\end{table}",
            );
        }

        let nb_cols = self
            .table_data
            .first_lines
            .first()
            .expect("header present")
            .len();

        let mut tex = String::from("\\begin{table}[]\n\\begin{tabular}{|");
        tex.push_str("l|".repeat(nb_cols).as_str());
        tex.push_str("}\n\\hline\n");

        let mut sanitized_table = Vec::new();

        // Find maximum string len inside table to format it
        let max_size = {
            let mut max = 0;
            for (id, line) in self.table_data.first_lines.iter().enumerate() {
                sanitized_table.push(Vec::new());

                line.iter().for_each(|x| {
                    let sanitized_val = sanitize_latex(x, id == 0);
                    max = max.max(sanitized_val.len());
                    sanitized_table[id].push(sanitized_val);
                });
            }

            if self.table_data.has_overflown {
                let dots = String::from("$\\cdots$");
                sanitized_table.push((0..nb_cols).map(|_| dots.clone()).collect());
                max = max.max(dots.len());
            }

            for line in &self.table_data.last_lines {
                sanitized_table.push(Vec::new());
                line.iter().for_each(|x| {
                    let sanitized_val = sanitize_latex(x, false);
                    max = max.max(sanitized_val.len());
                    sanitized_table
                        .last_mut()
                        .expect("one line added")
                        .push(sanitized_val);
                });
            }
            max
        };

        for line in &sanitized_table {
            tex.push_str(&to_latex_row(line, max_size));
        }

        tex.push_str("\\end{tabular}\n\\end{table}");
        tex
    }

    fn build_table(&self) -> Table {
        let mut builder = Builder::default();
        if !self.header_added {
            builder.push_record(vec!["Empty table"]);
        }
        if self.table_data.first_lines.is_empty() && self.table_data.last_lines.is_empty() {
            builder.push_record(vec!["     /     "]);
        }
        self.mode.build(&self.table_data, &mut builder);

        builder.build()
    }
}

fn sanitize_latex(value: &str, bold: bool) -> String {
    let mut san_value = String::new();

    value.chars().for_each(|c| {
        let c_str = c.to_string();
        san_value.push_str(match &c {
            '_' => "\\_",
            '%' => "\\%",
            '{' => "\\{",
            '}' => "\\}",
            '$' => "\\$",
            '\\' => "\\textbackslash{}",
            '^' => "\\textasciicircum{}",
            '~' => "\\textasciitilde{}",
            _ => &c_str,
        });
    });

    if bold {
        format!("\\textbf{{{san_value}}}")
    } else {
        san_value
    }
}

fn to_latex_row(line: &[String], fill_len: usize) -> String {
    let mut latex_line = String::new();
    for val in line.iter().take(line.len() - 1) {
        latex_line.push_str(&format!("{:}{} & ", val, " ".repeat(fill_len - val.len())));
    }
    let last_line = line.last().expect("one line");
    latex_line.push_str(&format!(
        "{:}{} \\\\ \\hline\n",
        last_line,
        " ".repeat(fill_len - last_line.len())
    ));

    latex_line
}

impl SaveOutput for QueryTable {
    /// Push the given lines in the table, while respecting the options of the table
    fn push_line<T: Into<String> + Clone>(&mut self, values: Vec<T>) {
        let mut values_indexed = if !self.header_added {
            self.header_added = true;
            vec![String::from("i")]
        } else {
            self.curr_index += 1;
            vec![(self.curr_index - 1).to_string()]
        };

        values_indexed.append(&mut to_vec_string(values));
        self.mode.add_line(&mut self.table_data, values_indexed);
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
            let mut t = self.build_table();
            t.with(Style::rounded());
            t.to_string()
        })
    }
}
