use std::{collections::HashMap, sync::LazyLock};

use pest::{
    Parser,
    error::{Error, LineColLocation},
    iterators::{Pair, Pairs},
    pratt_parser::PrattParser,
};
use pest_derive::Parser;
use thiserror::Error;

use crate::{
    data_handler::invariant_execs::Module,
    parser::parsed_expression::{ArithmOp, Comparison, Condition, MathExpression, ParsedArgType},
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
    // #[error("Error when creating the class selection \"{0}\", reason : \"{1}\"")]
    // ClassSelectionError(String, ClassSelectionError),
}

impl ParsingError {
    /// Gets a prettified version of the error.
    /// With the missing token if any and a visual indication of where the problem was encountered.
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
                        format!(", because one of these tokens could be missing: \"{token}\"")
                    } else {
                        String::new()
                    },
                    Self::get_arrow_under(input, *col_pos)
                )
            }
            _ => self.to_string(),
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
            negatives: _,
        } => {
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
/// Used to parse inputs for queries.
pub struct QueryParser;

static PRATT_PARSER: LazyLock<PrattParser<Rule>> = LazyLock::new(|| {
    use pest::pratt_parser::{Assoc::*, Op};

    // Precedence is defined lowest to highest
    PrattParser::new()
        // Addition and subtract have equal precedence
        .op(Op::infix(Rule::add, Left) | Op::infix(Rule::subtract, Left))
        .op(Op::infix(Rule::multiply, Left)
            | Op::infix(Rule::divide, Left)
            | Op::infix(Rule::floor_divide, Left)
            | Op::infix(Rule::modulo, Left))
        .op(Op::infix(Rule::exponent, Right))
        .op(Op::prefix(Rule::negation)
            | Op::prefix(Rule::abs)
            | Op::prefix(Rule::floor)
            | Op::prefix(Rule::ceil)
            | Op::prefix(Rule::sqrt))
});

impl QueryParser {
    // /// Parses a given query into one of the available queries. See [`ParsedQuery`] for more information.
    // pub fn parse_query(
    //     input: impl ToString,
    //     epsilon: Option<f64>,
    // ) -> Result<ParsedQuery, ParsingError> {
    //     let input_str = input.to_string();
    //     let input = Self::parse(Rule::query, &input_str);
    //     match input {
    //         Ok(mut rules) => {
    //             let mut rules = rules
    //                 .next()
    //                 .expect("first rule should be a query rule")
    //                 .into_inner();

    //             let inner_rule = rules.next().expect("always at least one subrule");
    //             match inner_rule.as_rule() {
    //                 Rule::if_query => Self::parse_if_query_rule(inner_rule, epsilon),
    //                 Rule::extremal_condition => Ok(ParsedQuery::Condition(
    //                     Self::parse_extremal_condition_rule(inner_rule, epsilon)?,
    //                 )),
    //                 _ => unreachable!(),
    //             }
    //         }
    //         Err(e) => Err(get_parsing_error(e)),
    //     }
    // }

    pub fn parse_input(
        input: impl ToString,
        epsilon: Option<f64>,
        fn_map: &HashMap<String, Module>,
    ) -> Result<(), ParsingError> {
        let condition = Self::parse_condition(input, epsilon)?;

        // Check compatibility

        Ok(())
    }
    /// Parses a condition using a string value into an equivalent [`Condition`].
    /// For example : `(a = 2 or not(x < y))`
    pub fn parse_condition(
        input: impl ToString,
        epsilon: Option<f64>,
    ) -> Result<Condition, ParsingError> {
        let input = input.to_string();
        let input = match QueryParser::parse(Rule::condition, &input) {
            Ok(mut input) => input.next().expect("one present"),
            Err(e) => {
                return Err(get_parsing_error(e));
            }
        };

        Ok(Self::parse_condition_rule(input, epsilon))
    }

    // /// Parses a conjecture query into an equivalent [`ExtremalCounterQuery`]
    // /// For example : `min(p_gn: n,m), d_nm >= 3 => conj1 = 1`
    // pub fn parse_extremal_condition(
    //     input: impl ToString,
    //     epsilon: Option<f64>,
    // ) -> Result<ParsedConditionDEPRECATED, ParsingError> {
    //     let input_str = input.to_string();
    //     let input = match QueryParser::parse(Rule::extremal_condition_eoi, &input_str) {
    //         Ok(mut input) => input.next().expect("one present"),
    //         Err(e) => {
    //             return Err(get_parsing_error(e));
    //         }
    //     };

    //     Self::parse_extremal_condition_rule(input, epsilon)
    // }

    pub fn parse_expression(input: impl ToString) -> Result<MathExpression, ParsingError> {
        match Self::parse(Rule::expr, &input.to_string()) {
            Ok(rules) => Ok(Self::parse_expr_rule(rules)),
            Err(e) => Err(get_parsing_error(e)),
        }
    }

    // fn parse_if_query_rule(
    //     rule: Pair<'_, Rule>,
    //     epsilon: Option<f64>,
    // ) -> Result<ParsedQuery, ParsingError> {
    //     let mut inner_rules = rule.into_inner();

    //     // Two tokens : `extremal_condition` and `condition`
    //     let extremal_cond = Self::parse_extremal_condition_rule(
    //         inner_rules.next().expect("extremal_condition present"),
    //         epsilon,
    //     )?;

    //     let condition =
    //         Self::parse_condition_rule(inner_rules.next().expect("condition present"), epsilon);

    //     Ok(ParsedQuery::IfElse(extremal_cond, condition))
    // }

    // fn parse_extremal_condition_rule(
    //     rule: Pair<'_, Rule>,
    //     epsilon: Option<f64>,
    // ) -> Result<ParsedConditionDEPRECATED, ParsingError> {
    //     let mut inner_rules = rule.into_inner();

    //     /*
    //     Three possibilities here :
    //     * `condition`
    //     * `extremal`
    //     * `extremal and_op condition`
    //      */
    //     if inner_rules.len() == 3 {
    //         // get `extremal`
    //         let extremal_selection =
    //             QueryParser::parse_extremal(inner_rules.next().expect("extremal present"))?;
    //         // skip `and_op`
    //         inner_rules.next();
    //         // get `condition`
    //         let condition = QueryParser::parse_condition_rule(
    //             inner_rules.next().expect("condition present"),
    //             epsilon,
    //         );
    //         return Ok(ParsedConditionDEPRECATED::ExtremalCondition(
    //             extremal_selection,
    //             Some(condition),
    //         ));
    //     }
    //     let first_rule = inner_rules
    //         .next()
    //         .expect("either extremal or condition rule");

    //     match first_rule.as_rule() {
    //         Rule::extremal => Ok(ParsedConditionDEPRECATED::ExtremalCondition(
    //             Self::parse_extremal(first_rule)?,
    //             None,
    //         )),
    //         Rule::condition => Ok(ParsedConditionDEPRECATED::Condition(
    //             Self::parse_condition_rule(first_rule, epsilon),
    //         )),
    //         _ => unreachable!(),
    //     }
    // }

    fn parse_condition_rule(rule: Pair<'_, Rule>, epsilon: Option<f64>) -> Condition {
        let mut inner_rules = rule.into_inner();
        if inner_rules.len() == 1 {
            // comparison
            let inner_rule = inner_rules.next().expect("one value");
            Self::parse_comparison_rule(inner_rule, epsilon)
        } else {
            let inner = inner_rules.next().expect("comp present");
            // comparison ~ binary_op ~ condition
            let comparison = Self::parse_comparison_rule(inner, epsilon);

            let binary_op = inner_rules
                .next()
                .expect("binary operator present")
                .into_inner()
                .next()
                .expect("one sub rule value");

            let condition =
                Self::parse_condition_rule(inner_rules.next().expect("condition present"), epsilon);
            create_condition(comparison, binary_op, condition)
        }
    }

    fn parse_comparison_rule(rule: Pair<'_, Rule>, epsilon: Option<f64>) -> Condition {
        let mut inner_rules = rule.into_inner();
        if inner_rules.len() == 1 {
            // either a "not_comparison" or a "condition".
            let inner_rule = inner_rules.next().expect("one present");
            match &inner_rule.as_rule() {
                Rule::not_comparison => {
                    // made up of a condition
                    // parse inner condition or identifier
                    let inner_rule = inner_rule.into_inner().next().expect("at least one");
                    match inner_rule.as_rule() {
                        // FIXME: Add back this shortcut later when the structure is more stable
                        // !inv is a shortcut for inv = 0 (reduces query sizes)
                        // Rule::identifier => SqlComparison::Equal(
                        //     ArgType::identifier(inner_rule.as_str()).into(),
                        //     ArgType::value(0).into(),
                        //     None,
                        // )
                        // .into(),
                        Rule::condition => {
                            Condition::not(Self::parse_condition_rule(inner_rule, epsilon))
                        }
                        _ => {
                            unreachable!()
                        }
                    }
                }
                Rule::condition => Self::parse_condition_rule(inner_rule, epsilon),
                Rule::function => Comparison::Equal(
                    parse_function(inner_rule).into(),
                    ParsedArgType::prim_numeric(1.).into(),
                    None,
                )
                .into(),
                Rule::identifier => Comparison::Equal(
                    ParsedArgType::invariant(inner_rule.as_str()).into(),
                    ParsedArgType::prim_numeric(1.).into(),
                    None,
                )
                .into(),
                _ => unreachable!("{inner_rule:?}"),
            }
        } else {
            // expression - comp_operator - expression
            let prim_1 =
                Self::parse_expr_rule(inner_rules.next().expect("prim1 present").into_inner());
            let operator = inner_rules
                .next()
                .expect("operator present")
                .into_inner()
                .next()
                .expect("one sub operator rule");
            let prim_2 =
                Self::parse_expr_rule(inner_rules.next().expect("prim2 present").into_inner());

            Condition::Operation(create_comparison(prim_1, operator, prim_2, epsilon))
        }
    }

    // fn parse_extremal(rule: Pair<'_, Rule>) -> Result<ClassSelection, ParsingError> {
    //     let rule_str = rule.as_str();
    //     let inner_rules = rule.into_inner();

    //     let mut first_identifier = None;
    //     let mut identifiers = Vec::new();
    //     let mut class_type = ClassType::Max;
    //     for inner_rule in inner_rules {
    //         match &inner_rule.as_rule() {
    //             Rule::extremal_func => {
    //                 let inner_rule = inner_rule.into_inner().next().expect("at least one");
    //                 match &inner_rule.as_rule() {
    //                     Rule::max_func => {
    //                         class_type = ClassType::Max;
    //                     }
    //                     Rule::min_func => {
    //                         class_type = ClassType::Min;
    //                     }
    //                     _ => unreachable!(),
    //                 }
    //             }
    //             Rule::identifier => {
    //                 // TODO: not sure this is correct but oh well
    //                 let new_prim = ParsedArgType::invariant(inner_rule.as_str());
    //                 if first_identifier.is_none() {
    //                     first_identifier = Some(new_prim)
    //                 } else {
    //                     identifiers.push(new_prim);
    //                 }
    //             }
    //             _ => unreachable!(),
    //         }
    //     }
    //     match ClassSelection::new(
    //         class_type,
    //         first_identifier.expect("at least one"),
    //         identifiers,
    //     ) {
    //         Ok(val) => Ok(val),
    //         Err(e) => Err(ParsingError::ClassSelectionError(rule_str.to_string(), e)),
    //     }
    // }

    fn parse_expr_rule(pairs: Pairs<Rule>) -> MathExpression {
        PRATT_PARSER
            // One value
            .map_primary(|primary| match primary.as_rule() {
                Rule::primitif => MathExpression::Primitif(create_primitif(primary)),
                Rule::expr => Self::parse_expr_rule(primary.into_inner()),
                rule => unreachable!("Expr::parse expected atom, found {:?}", rule),
            })
            // Three values
            .map_infix(|lhs, op, rhs| {
                let op = match op.as_rule() {
                    Rule::add => ArithmOp::Add,
                    Rule::subtract => ArithmOp::Subtract,
                    Rule::exponent => ArithmOp::Power,
                    Rule::multiply => ArithmOp::Multiply,
                    Rule::floor_divide => {
                        // A floor divide is actually the floor functions called on a division
                        return MathExpression::floor(MathExpression::bin_operation(
                            lhs,
                            super::parsed_expression::ArithmOp::Divide,
                            rhs,
                        ));
                    }
                    Rule::divide => ArithmOp::Divide,
                    Rule::modulo => ArithmOp::Modulo,
                    rule => unreachable!("Expr::parse expected infix operation, found {:?}", rule),
                };
                MathExpression::bin_operation(lhs, op, rhs)
            })
            // Two values (functions) ex: `abs(x)`
            .map_prefix(|op, rhs| match op.as_rule() {
                Rule::negation => MathExpression::negation(rhs),
                Rule::abs => MathExpression::abs(rhs),
                Rule::floor => MathExpression::floor(rhs),
                Rule::sqrt => MathExpression::sqrt(rhs),
                Rule::ceil => MathExpression::ceil(rhs),
                _ => unreachable!(),
            })
            .parse(pairs)
    }

    /// Replaces all aliases occurance with their given value.
    pub fn change_aliases(mut og_query: String, aliases: &[(String, String)]) -> String {
        for (alias, value) in aliases {
            og_query = og_query.replace(alias, &format!("({value})"));
        }

        og_query
    }
}

fn create_condition(
    cond_1: Condition,
    binary_op_rule: Pair<'_, Rule>,
    cond_2: Condition,
) -> Condition {
    match binary_op_rule.as_rule() {
        Rule::and_op => Condition::and(cond_1, cond_2),
        Rule::or_op => Condition::or(cond_1, cond_2),
        Rule::xor_op => Condition::xor(cond_1, cond_2),
        Rule::implication_op => Condition::implication(cond_1, cond_2),
        Rule::equivalence_op => Condition::equivalence(cond_1, cond_2),
        _ => unreachable!(),
    }
}

fn parse_function(fn_rule: Pair<'_, Rule>) -> ParsedArgType {
    let mut inner_rule = fn_rule.into_inner();
    // Function name
    let fn_name = inner_rule
        .next()
        .expect("fn identifier present")
        .as_str()
        .to_string();
    // First argument
    let first_arg =
        QueryParser::parse_expr_rule(inner_rule.next().expect("first arg present").into_inner());
    // zero or more arguments
    let mut args = Vec::new();
    for rule in inner_rule {
        args.push(QueryParser::parse_expr_rule(rule.into_inner()));
    }
    ParsedArgType::function(fn_name, first_arg, args)
}

fn create_primitif(prim_rule: Pair<'_, Rule>) -> ParsedArgType {
    let inner_rule = prim_rule
        .into_inner()
        .next()
        .expect("at least one sub rule");
    match &inner_rule.as_rule() {
        Rule::function => parse_function(inner_rule),
        // Shorthand for invariants or for the graph G identifier
        Rule::identifier => {
            // Either a function or the identifier of a graph.
            let str_val = inner_rule.as_str().to_string();
            if let Some(g) = inner_rule.into_inner().next()
                && let Rule::graph = g.as_rule()
            {
                ParsedArgType::Graph
            } else {
                ParsedArgType::invariant(str_val)
            }
        }
        Rule::value => {
            // either a number of string value:
            let inner_rule = inner_rule.into_inner().next().expect("at least one");
            match inner_rule.as_rule() {
                Rule::number => ParsedArgType::PrimNumeric({
                    inner_rule
                        .as_str()
                        .trim()
                        .parse::<f64>()
                        .expect("correct f64 value")
                }),
                Rule::string => {
                    let value = inner_rule.as_str().replace("\"", "");
                    ParsedArgType::PrimString(value.trim().to_string())
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
}

fn create_comparison(
    prim_1: MathExpression,
    binary_op_rule: Pair<'_, Rule>,
    prim_2: MathExpression,
    epsilon: Option<f64>,
) -> Comparison {
    match binary_op_rule.as_rule() {
        Rule::equal => Comparison::Equal(prim_1, prim_2, epsilon),
        Rule::not_equal => Comparison::NotEqual(prim_1, prim_2, epsilon),
        Rule::less => Comparison::Less(prim_1, prim_2),
        Rule::less_equal => Comparison::LessEqual(prim_1, prim_2, epsilon),
        Rule::greater => Comparison::Greater(prim_1, prim_2),
        Rule::greater_equal => Comparison::GreaterEqual(prim_1, prim_2, epsilon),
        _ => unreachable!(),
    }
}
