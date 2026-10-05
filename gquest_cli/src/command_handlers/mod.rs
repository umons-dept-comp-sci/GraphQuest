use gquest_core::utils::{
    SaveOutput,
    csv_utils::CsvFile,
    table_handler::{QueryTable, QueryTableOptions},
};

use crate::{
    CliError,
    cli_commands::{OutputSettings, TableOutputFormat},
    command_handlers::arg_parser::ArgParser,
};

pub mod add_remove;
pub mod arg_parser;
pub mod query;

pub struct OutputHandler {
    file: Option<CsvFile>,
    table: QueryTable,
    format: TableOutputFormat,
}

impl OutputHandler {
    pub fn new(settings: OutputSettings) -> Result<Self, CliError> {
        let file = match settings.output_file.output_path {
            Some(path) => Some(CsvFile::new_no_headers(
                &path,
                Some(settings.output_file.separator),
            )?),
            None => None,
        };

        let options = match settings.partial {
            Some(partial) => ArgParser::parse_partial_table(&partial)?,
            None => QueryTableOptions::Full,
        };

        Ok(Self {
            file,
            table: QueryTable::new_no_header(!settings.no_id, options),
            format: settings.format,
        })
    }

    pub fn close(self) -> String {
        match self.format {
            TableOutputFormat::Table => self.table.to_string(),
            TableOutputFormat::PlainText => self.table.to_plaintext(),
            TableOutputFormat::Latex => self.table.to_latex(),
            TableOutputFormat::Markdown => self.table.to_markdown(),
        }
    }
}

impl SaveOutput for OutputHandler {
    fn push_line<T: Into<String> + Clone>(&mut self, values: Vec<T>) {
        if let Some(file) = &mut self.file {
            file.write_lines_to_file(Vec::from([values.clone()]))
                .expect("no issues writing to file")
        }

        self.table
            .push_line(values.into_iter().map(|v| v.into()).collect());
    }
}
