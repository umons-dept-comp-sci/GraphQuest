use pest::{
    Parser,
    iterators::{Pair, Pairs},
};
use pest_derive::Parser;
use sqlx::Sqlite;

use crate::database_handler::{
    ArgType, ClassSelection, ClassType, ExtremalCounterExampleQuery, SqlComparison, SqlCondition,
};

#[derive(Parser)]
#[grammar = "parser/grammar.pest"] // relative to src
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
    // let comparison = QueryParser::parse(Rule::condition, "((a_1 = x) or not(\"1\"!=4)) and p >= 4")
    //     .expect("correct")
    //     .next()
    //     .expect("correct");

    // let res = QueryParser::parse_condition(comparison);

    // println!("{}", res.to_sql::<Sqlite>())
    
    let extremal = QueryParser::parse(Rule::extremal, "min(a: b, c,d)").expect("correct").next().expect("one val");

    let res = QueryParser::parse_extremal(extremal);

    println!("{:?}", res)
}
