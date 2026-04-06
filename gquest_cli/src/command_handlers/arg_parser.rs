use std::collections::HashSet;

use gquest_core::utils::table_handler::QueryTableOptions;
use pest::{
    Parser,
    error::{Error, ErrorVariant, LineColLocation},
    iterators::Pair,
};
use pest_derive::Parser;

use crate::CliError;

#[derive(Parser)]
#[grammar = "command_handlers/grammar.pest"]
pub struct ArgParser;

impl ArgParser {
    pub fn parse_order(input: &str) -> Result<Vec<u32>, CliError> {
        match ArgParser::parse(Rule::order, input) {
            Ok(mut rule) => Ok(ArgParser::parse_order_rule(
                rule.next().expect("one sub rule"),
            )),
            Err(e) => Err(Self::get_error(e)),
        }
    }

    pub fn parse_partial_table(input: &str) -> Result<QueryTableOptions, CliError> {
        match ArgParser::parse(Rule::partial_table, input) {
            Ok(mut rule) => Ok(ArgParser::parse_partial_tablerule(
                rule.next().expect("one sub rule"),
            )),
            Err(e) => Err(Self::get_error(e)),
        }
    }

    pub fn get_error(e: Error<Rule>) -> CliError {
        let arg = e.line().to_string();
        let column = match e.line_col {
            LineColLocation::Pos((_, col)) => col,
            _ => unreachable!(),
        };
        // Collect possible missing tokens
        let missing_tokens: Vec<String> = match e.variant {
            ErrorVariant::ParsingError {
                positives,
                negatives: _,
            } => positives
                .iter()
                .map(|rule| {
                    match rule {
                        Rule::colon => ":",
                        Rule::comma => ",",
                        Rule::int => "integer",
                        _ => "",
                    }
                    .to_string()
                })
                .collect(),
            _ => unreachable!(),
        };

        CliError::ArgParseError {
            arg,
            column,
            missing_tokens,
        }
    }

    fn parse_partial_tablerule(rule: Pair<'_, Rule>) -> QueryTableOptions {
        let inner_rule = rule.into_inner().next().expect("one sub rule");

        let rule = inner_rule.as_rule();
        let mut inner_rules = inner_rule.into_inner();
        match rule {
            Rule::partial => {
                let first_rows_count =
                    Self::parse_int_rule(inner_rules.next().expect("present")) as usize;
                inner_rules.next();
                let last_rows_count =
                    Self::parse_int_rule(inner_rules.next().expect("present")) as usize;

                QueryTableOptions::Partial {
                    first_rows_count,
                    last_rows_count,
                }
            }
            Rule::partial_no_end => QueryTableOptions::Partial {
                first_rows_count: Self::parse_int_rule(inner_rules.next().expect("present"))
                    as usize,
                last_rows_count: 0,
            },
            Rule::partial_no_start => {
                inner_rules.next();

                QueryTableOptions::Partial {
                    first_rows_count: 0,
                    last_rows_count: Self::parse_int_rule(inner_rules.next().expect("present"))
                        as usize,
                }
            }
            _ => unreachable!(),
        }
    }

    fn parse_order_rule(rule: Pair<'_, Rule>) -> Vec<u32> {
        let inner_rule = rule.into_inner().next().expect("one sub rule");
        let mut range = HashSet::new();
        match &inner_rule.as_rule() {
            Rule::int_list => {
                for int_value in inner_rule.into_inner() {
                    match &int_value.as_rule() {
                        Rule::int => {
                            range.insert(Self::parse_int_rule(int_value));
                        }
                        Rule::comma => {}
                        _ => unreachable!(),
                    }
                }
            }
            Rule::int_range_list => {
                for int_range in inner_rule.into_inner() {
                    match &int_range.as_rule() {
                        Rule::int_range => {
                            let mut int_range_rules = int_range.into_inner();
                            // int range is always : int ~ ":" ~ int
                            let start = Self::parse_int_rule(
                                int_range_rules.next().expect("first int present"),
                            );
                            int_range_rules.next(); // skip colon
                            let end = Self::parse_int_rule(
                                int_range_rules.next().expect("last int present"),
                            );

                            range.extend(start..end + 1);
                        }
                        Rule::comma => {}
                        _ => unreachable!(),
                    }
                }
            }
            _ => unreachable!(),
        };
        Vec::from_iter(range)
    }

    fn parse_int_rule(rule: Pair<'_, Rule>) -> u32 {
        rule.as_str().parse::<u32>().expect("correct integer")
    }
}
