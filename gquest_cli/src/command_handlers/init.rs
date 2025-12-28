use std::collections::HashSet;

use gquest_core::{
    data_handler::data_loader::{GengProcess, read_file, read_pipe_signatures},
    database_handler::{GraphDatabase, SqliteGraphDB},
};
use log::{error, info};
use pest::{
    Parser,
    error::{ErrorVariant, LineColLocation},
    iterators::{Pair, Pairs},
};
use pest_derive::Parser;

use crate::{
    CliError,
    cli_commands::{DatabasePath, DatasetChoice, GengArgs},
    progress_bar::GquestProgressBar,
};

/// Creates a database (or simply connects to the existant one) and adds a dataset using the prefered way of the user
pub async fn add_dataset(path: DatabasePath, input_method: DatasetChoice) -> Result<(), CliError> {
    info!("Creating/connecting database at path : {:?}", path.url);
    let mut db = match SqliteGraphDB::connect_create_graph_database(path.url, None).await {
        Ok(db) => db,
        Err(e) => {
            error!("Error creating/connecting to the given db: {e}");
            return Ok(());
        }
    };
    info!("Importing given dataset");
    let mut pb = GquestProgressBar::new(
        crate::progress_bar::ProgressBarType::Iterating {
            start: 0,
            length: 12005168,
        },
        Some("Cool"),
    );

    match input_method {
        DatasetChoice::Geng { args } => {
            // Parse input
            for order in OrderParser::parse_order(&args.order)? {
                println!("{order}");
                let geng_call = GengProcess::call_geng(order, &String::new(), (None, None))
                    .expect("correct geng");

                db.add_to_dataset(geng_call.get_reader(), args.batch_size, Some(&mut pb))
                    .await.expect("erro");
            }
        }

        DatasetChoice::File { path } => {
            // let file = match read_file(&path) {
            //     Ok(f) => f,
            //     Err(e) => {
            //         error!("Could not open the given file : {e}");
            //         return;
            //     }
            // };

            // db.add_to_dataset(file, 100, Some(&mut pb)).await;
        }
        DatasetChoice::Pipe => {
            db.add_to_dataset(read_pipe_signatures(), 100, Some(&mut pb))
                .await;
        }
    };
    pb.force_finish();
    info!("Closing database");
    db.close_connection().await;
    Ok(())
}

#[derive(Parser)]
#[grammar = "command_handlers/grammar.pest"]
struct OrderParser;

impl OrderParser {
    fn parse_order(input: &str) -> Result<Vec<u32>, CliError> {
        match OrderParser::parse(Rule::order, input) {
            Ok(mut rule) => Ok(OrderParser::parse_order_rule(
                rule.next().expect("one present"),
            )),
            Err(e) => {
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

                Err(CliError::ArgParseError {
                    arg: input.to_string(),
                    column,
                    missing_tokens,
                })
            }
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

                            range.extend(start..end+1);
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
