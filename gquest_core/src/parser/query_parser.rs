use pest::{
    Parser,
    error::{Error, LineColLocation},
    iterators::Pair,
};
use pest_derive::Parser;
use sqlx::Sqlite;
use thiserror::Error;

use crate::database_handler::{
    ArgType, ClassSelection, ClassType, ExtremalCounterExampleQuery, SqlComparison, SqlCondition,
};

#[derive(Debug, Error)]
pub enum ParsingError {
    #[error("Something went wrong when trying to parse the given input : \"{input}\"")]
    ParseError {
        /// The input that caused the error
        input: String,
        /// The index of the column where the parse error originated
        col_pos: usize, // No need for line position as they are always one line
        /// The missing token that could possibly fix the error
        missing_token: Option<String>,
    },
}

impl ParsingError {
    pub fn pretty_string(&self) -> String {
        match self {
            ParsingError::ParseError {
                input,
                col_pos,
                missing_token,
            } => {
                format!(
                    "Could not parse the following input{} : \n{}",
                    if let Some(token) = missing_token {
                        format!(", this could be missing: \"{token}\"")
                    } else {
                        String::new()
                    },
                    Self::get_arrow_under(input, *col_pos)
                )
            }
        }
    }
    fn get_arrow_under(value: &String, col: usize) -> String {
        format!(" {value}\n{}^", String::from("-").repeat(col))
    }
}

fn get_parsing_error(error: Error<Rule>) -> ParsingError {
    let input = error.line();
    let col = match error.line_col {
        LineColLocation::Pos((_, col)) => col,
        LineColLocation::Span(_, _) => todo!("impl span error"),
    };
    let mut missing_token = None;
    match &error.variant {
        pest::error::ErrorVariant::ParsingError {
            positives,
            negatives,
        } => {
            println!("positives: {positives:?}");
            println!("negatives: {negatives:?}");
            for r in positives {
                missing_token = match r {
                    Rule::binary_op => Some(String::from("AND, OR")),
                    Rule::comp_operator => Some(String::from(">=, <=, =, !=, ...")),
                    Rule::primitif => Some(String::from("an identifier or a value")),
                    _ => None,
                };
                if missing_token.is_some() {
                    break;
                }
            }
        }
        pest::error::ErrorVariant::CustomError { message: _ } => {
            unreachable!("This should never be triggered")
        }
    }

    ParsingError::ParseError {
        input: input.to_string(),
        col_pos: col,
        missing_token,
    }
}

#[derive(Parser)]
#[grammar = "parser/grammar.pest"] // relative to src
/// Used to parse inputs for conjecture queries.
struct QueryParser;

impl QueryParser {
    fn parse_condition(rule: Pair<'_, Rule>) -> SqlCondition {
        let mut inner_rules = rule.into_inner();
        // println!("len {}, {inner_rules}", inner_rules.len());
        if inner_rules.len() == 1 {
            // comparison
            let inner_rule = inner_rules.next().expect("one value");
            Self::parse_comparison(inner_rule)
        } else {
            // comparison ~ binary_op ~ condition
            let mut comparison = None;
            let mut binary_op = None;
            let mut condition = None;
            for inner_rule in inner_rules {
                match &inner_rule.as_rule() {
                    Rule::comparison => comparison = Some(Self::parse_comparison(inner_rule)),
                    Rule::binary_op => {
                        binary_op = Some(inner_rule.into_inner().next().expect("one value"));
                    }
                    Rule::condition => condition = Some(Self::parse_condition(inner_rule)),
                    _ => unreachable!(),
                }
            }
            create_condition(
                comparison.expect("present"),
                binary_op.expect("present"),
                condition.expect("present"),
            )
        }
    }

    fn parse_comparison(rule: Pair<'_, Rule>) -> SqlCondition {
        let mut inner_rules = rule.into_inner();
        if inner_rules.len() == 1 {
            // either a "not_comparison" or a "condition".
            let inner_rule = inner_rules.next().expect("one present");
            match &inner_rule.as_rule() {
                Rule::not_comparison => {
                    // made up of a condition
                    // parse inner condition
                    SqlCondition::not(Self::parse_condition(
                        inner_rule.into_inner().next().expect("at least one"),
                    ))
                }
                Rule::condition => Self::parse_condition(inner_rule),
                _ => unreachable!(),
            }
        } else {
            let mut prims = Vec::with_capacity(2);
            let mut operator = None;
            for rule in inner_rules {
                // primitif - comp_operator - primitif
                match &rule.as_rule() {
                    Rule::primitif => {
                        prims.push(create_primitif(
                            rule.into_inner().next().expect("at least one val"),
                        ));
                    }
                    Rule::comp_operator => {
                        operator = Some(rule.into_inner().next().expect("one operator"));
                    }
                    _ => unreachable!(),
                }
            }

            let prim_2 = prims.pop().expect("two values");
            let prim_1 = prims.pop().expect("two values");

            SqlCondition::Operation(create_comparison(
                prim_1,
                operator.expect("a value"),
                prim_2,
            ))
        }
    }

    fn parse_extremal(rule: Pair<'_, Rule>) -> ClassSelection {
        let inner_rules = rule.into_inner();

        let mut first_identifier = None;
        let mut identifiers = Vec::new();
        let mut class_type = ClassType::Max;
        for inner_rule in inner_rules {
            match &inner_rule.as_rule() {
                Rule::extremal_func => {
                    let inner_rule = inner_rule.into_inner().next().expect("at least one");
                    match &inner_rule.as_rule() {
                        Rule::max_func => {
                            class_type = ClassType::Max;
                        }
                        Rule::min_func => {
                            class_type = ClassType::Min;
                        }
                        _ => unreachable!(),
                    }
                }
                Rule::identifier => {
                    let new_prim = create_primitif(inner_rule);
                    if first_identifier.is_none() {
                        first_identifier = Some(new_prim)
                    } else {
                        identifiers.push(new_prim);
                    }
                }
                _ => unreachable!(),
            }
        }
        ClassSelection::new(
            class_type,
            first_identifier.expect("at least one"),
            identifiers,
        )
    }

    fn parse_conj_query(rule: Pair<'_, Rule>) -> ExtremalCounterExampleQuery {
        let mut inner_rules = rule.into_inner();

        let selection = Self::parse_extremal(inner_rules.next().expect("extramal present"));
        let additional_condition = if inner_rules.len() == 3 {
            Some(Self::parse_condition(
                inner_rules.next().expect("extramal present"),
            ))
        } else {
            None
        };

        let conjecture_to_disprove =
            Self::parse_condition(inner_rules.next().expect("extramal present"));

        ExtremalCounterExampleQuery {
            selection,
            additional_condition,
            conjecture_to_disprove,
        }
    }
}

fn create_condition(
    cond_1: SqlCondition,
    binary_op_rule: Pair<'_, Rule>,
    cond_2: SqlCondition,
) -> SqlCondition {
    match binary_op_rule.as_rule() {
        Rule::and_op => SqlCondition::and(cond_1, cond_2),
        Rule::or_op => SqlCondition::or(cond_1, cond_2),
        _ => unreachable!(),
    }
}

fn create_primitif(prim_rule: Pair<'_, Rule>) -> ArgType {
    match &prim_rule.as_rule() {
        Rule::identifier => ArgType::Identifier(prim_rule.as_str().to_string()),
        Rule::value => {
            // either a number of string value:
            let inner_rule = prim_rule.into_inner().next().expect("at least one");
            match inner_rule.as_rule() {
                Rule::number => ArgType::Value(inner_rule.as_str().trim().to_string()),
                Rule::string => {
                    let value = inner_rule.as_str().replace("\"", "");
                    ArgType::Value(value.trim().to_string())
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
}

fn create_comparison(
    prim_1: ArgType,
    binary_op_rule: Pair<'_, Rule>,
    prim_2: ArgType,
) -> SqlComparison {
    match binary_op_rule.as_rule() {
        Rule::equal => SqlComparison::Equal(prim_1, prim_2),
        Rule::not_equal => SqlComparison::NotEqual(prim_1, prim_2),
        Rule::less => SqlComparison::Less(prim_1, prim_2),
        Rule::less_equal => SqlComparison::LessEqual(prim_1, prim_2),
        Rule::greater => SqlComparison::Greater(prim_1, prim_2),
        Rule::greater_equal => SqlComparison::GreaterEqual(prim_1, prim_2),
        _ => unreachable!(),
    }
}

#[test]
fn HELP() {
    let comparison = QueryParser::parse(Rule::condition, "((a_1 = x) v !(\"1\"≠4)) ∧ p ≥ 4")
        .expect("correct")
        .next()
        .expect("correct");

    let res = QueryParser::parse_condition(comparison);

    println!("{}", res.to_sql::<Sqlite>());

    let extremal = QueryParser::parse(Rule::extremal, "min(a: b, c,d)")
        .expect("correct")
        .next()
        .expect("one val");

    let extremal = QueryParser::parse(Rule::conj_query, "min(a: b, c,d), a =b => conj_1 = 0")
        .expect("correct")
        .next()
        .expect("one val");

    let res = QueryParser::parse_conj_query(extremal);

    println!("{:?}", res);

    if let Err(e) = QueryParser::parse(Rule::conj_query, "min(a: b, c,d), a = b => conj_1 = 1") {
        println!("{}", get_parsing_error(e).pretty_string());
    }
}
