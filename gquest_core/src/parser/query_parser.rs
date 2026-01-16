use std::sync::LazyLock;

use pest::{
    Parser,
    error::{Error, LineColLocation},
    iterators::{Pair, Pairs},
    pratt_parser::PrattParser,
};
use pest_derive::Parser;
use thiserror::Error;

use crate::database_handler::{
    ArgType, ArithmOp, ClassSelection, ClassSelectionError, ClassType, ExtremalCounterQuery,
    MathExpression, SqlComparison, SqlCondition,
};

/// Represents the available queries that can be parsed and submitted to the graph database.
pub enum ParsedQuery {
    /// A simple condition to apply to the database.
    Condition(SqlCondition),
    /// An extremal selection of graphs with an optional condition.
    Extremal((ClassSelection, Option<SqlCondition>)),
    /// A counter example query for a given conjecture.
    Counter((SqlCondition, SqlCondition)),
    /// A counter example query for a given extremal conjecture.
    ExtremalCounter(ExtremalCounterQuery),
}

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
    #[error("Error when creating the class selection \"{0}\", reason : \"{1}\"")]
    ClassSelectionError(String, ClassSelectionError),
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
/// Used to parse inputs for conjecture queries.
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
        .op(Op::prefix(Rule::negation) | Op::prefix(Rule::abs))
    // .op(Op::prefix(unary_minus))
});

impl QueryParser {
    /// Parses a given query into one of the available queries. See [`ParsedQuery`] for more information.
    pub fn parse_query(input: impl ToString) -> Result<ParsedQuery, ParsingError> {
        let input_str = input.to_string();
        let input = Self::parse(Rule::query, &input_str);
        match input {
            Ok(mut rules) => {
                let mut rules = rules
                    .next()
                    .expect("first rule should be a query rule")
                    .into_inner();

                let inner_rule = rules.next().expect("always at least one subrule");
                match inner_rule.as_rule() {
                    Rule::condition => Ok(ParsedQuery::Condition(Self::parse_condition_rule(
                        inner_rule,
                    ))),
                    Rule::extremal_query => {
                        let inner_rule = inner_rule.into_inner().next().expect("one subrule");
                        Ok(ParsedQuery::Extremal(Self::parse_extremal_query_rule(
                            inner_rule,
                        )?))
                    }
                    Rule::extremal_conj_query => Ok(ParsedQuery::ExtremalCounter(
                        Self::parse_extremal_conj_query_rule(inner_rule)?,
                    )),
                    Rule::conj_query => Ok(ParsedQuery::Counter(Self::parse_conj_query_rule(
                        inner_rule,
                    )?)),
                    _ => unreachable!(),
                }
            }
            Err(e) => Err(get_parsing_error(e)),
        }
    }

    /// Parses a condition using a string value into an equivalent [`SqlCondition`].
    /// For example : `(a = 2 or not(x < y))`
    pub fn parse_condition(input: impl ToString) -> Result<SqlCondition, ParsingError> {
        let input = input.to_string();
        let input = match QueryParser::parse(Rule::condition, &input) {
            Ok(mut input) => input.next().expect("one present"),
            Err(e) => {
                return Err(get_parsing_error(e));
            }
        };

        Ok(Self::parse_condition_rule(input))
    }

    /// Parses a conjecture query into an equivalent [`ExtremalCounterQuery`]
    /// For example : `min(p_gn: n,m), d_nm >= 3 => conj1 = 1`
    pub fn parse_extr_conj_query(
        input: impl ToString,
    ) -> Result<ExtremalCounterQuery, ParsingError> {
        let input_str = input.to_string();
        let input = match QueryParser::parse(Rule::extremal_conj_query, &input_str) {
            Ok(mut input) => input.next().expect("one present"),
            Err(e) => {
                return Err(get_parsing_error(e));
            }
        };

        Self::parse_extremal_conj_query_rule(input)
    }

    /// Parses an extremal query.
    /// For example : `min(p_gn: n,m), d_nm >= 3
    pub fn parse_extremal_query(
        input: impl ToString,
    ) -> Result<(ClassSelection, Option<SqlCondition>), ParsingError> {
        let input_str = input.to_string();
        let input = match QueryParser::parse(Rule::extremal_query, &input_str) {
            Ok(mut input) => input.next().expect("one present"),
            Err(e) => {
                return Err(get_parsing_error(e));
            }
        };
        let inner_rule = input.into_inner().next().expect("one subrule");

        Self::parse_extremal_query_rule(inner_rule)
    }

    pub fn parse_expression(input: impl ToString) -> Result<MathExpression, ParsingError> {
        match Self::parse(Rule::expr, &input.to_string()) {
            Ok(rules) => Ok(Self::parse_expr_rule(rules)),
            Err(e) => Err(get_parsing_error(e)),
        }
    }

    fn parse_condition_rule(rule: Pair<'_, Rule>) -> SqlCondition {
        let mut inner_rules = rule.into_inner();
        if inner_rules.len() == 1 {
            // comparison
            let inner_rule = inner_rules.next().expect("one value");
            Self::parse_comparison_rule(inner_rule)
        } else {
            let inner = inner_rules.next().expect("comp present");
            // comparison ~ binary_op ~ condition
            let comparison = Self::parse_comparison_rule(inner);

            let binary_op = inner_rules
                .next()
                .expect("binary operator present")
                .into_inner()
                .next()
                .expect("one sub rule value");

            let condition =
                Self::parse_condition_rule(inner_rules.next().expect("condition present"));
            create_condition(comparison, binary_op, condition)
        }
    }

    fn parse_comparison_rule(rule: Pair<'_, Rule>) -> SqlCondition {
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
                        // !inv is a shortcut for inv = 0 (reduces query sizes)
                        Rule::identifier => SqlComparison::Equal(
                            ArgType::identifier(inner_rule.as_str()),
                            ArgType::value(0),
                        )
                        .into(),
                        Rule::condition => {
                            SqlCondition::not(Self::parse_condition_rule(inner_rule))
                        }
                        _ => {
                            unreachable!()
                        }
                    }
                }
                Rule::condition => Self::parse_condition_rule(inner_rule),
                Rule::identifier => SqlComparison::Equal(
                    ArgType::identifier(inner_rule.as_str()),
                    ArgType::value(1),
                )
                .into(),
                _ => unreachable!(),
            }
        } else {
            // primitif - comp_operator - primitif
            let prim_1 = create_primitif(inner_rules.next().expect("prim1 present"));
            let operator = inner_rules
                .next()
                .expect("operator present")
                .into_inner()
                .next()
                .expect("one sub operator rule");
            let prim_2 = create_primitif(inner_rules.next().expect("prim2 present"));

            SqlCondition::Operation(create_comparison(prim_1, operator, prim_2))
        }
    }

    fn parse_extremal(rule: Pair<'_, Rule>) -> Result<ClassSelection, ParsingError> {
        let rule_str = rule.as_str();
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
                    let new_prim = ArgType::identifier(inner_rule.as_str());
                    if first_identifier.is_none() {
                        first_identifier = Some(new_prim)
                    } else {
                        identifiers.push(new_prim);
                    }
                }
                _ => unreachable!(),
            }
        }
        match ClassSelection::new(
            class_type,
            first_identifier.expect("at least one"),
            identifiers,
        ) {
            Ok(val) => Ok(val),
            Err(e) => Err(ParsingError::ClassSelectionError(rule_str.to_string(), e)),
        }
    }

    fn parse_extremal_conj_query_rule(
        rule: Pair<'_, Rule>,
    ) -> Result<ExtremalCounterQuery, ParsingError> {
        let mut inner_rules = rule.into_inner();

        let (selection, additional_condition) =
            Self::parse_extremal_query_rule(inner_rules.next().expect("extremal query present"))?;

        let conjecture_to_disprove =
            Self::parse_condition_rule(inner_rules.next().expect("extramal present"));

        Ok(ExtremalCounterQuery {
            selection,
            additional_condition,
            conjecture_to_disprove,
        })
    }

    fn parse_conj_query_rule(
        rule: Pair<'_, Rule>,
    ) -> Result<(SqlCondition, SqlCondition), ParsingError> {
        let mut inner_rules = rule.into_inner();

        let left_condition =
            Self::parse_condition_rule(inner_rules.next().expect("extremal query present"));

        let right_condition =
            Self::parse_condition_rule(inner_rules.next().expect("extramal present"));

        Ok((left_condition, right_condition))
    }

    fn parse_extremal_query_rule(
        rule: Pair<'_, Rule>,
    ) -> Result<(ClassSelection, Option<SqlCondition>), ParsingError> {
        let mut inner_rules = rule.into_inner();
        let selection = Self::parse_extremal(inner_rules.next().expect("extramal present"))?;
        let additional_condition = if inner_rules.len() == 1 {
            Some(Self::parse_condition_rule(
                inner_rules.next().expect("condition present"),
            ))
        } else {
            None
        };

        Ok((selection, additional_condition))
    }

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
                        return MathExpression::Floor(Box::new(MathExpression::BinOperation {
                            left: Box::new(lhs),
                            op: ArithmOp::Divide,
                            right: Box::new(rhs),
                        }));
                    }
                    Rule::divide => ArithmOp::Divide,
                    Rule::modulo => ArithmOp::Modulo,
                    rule => unreachable!("Expr::parse expected infix operation, found {:?}", rule),
                };
                MathExpression::BinOperation {
                    left: Box::new(lhs),
                    op,
                    right: Box::new(rhs),
                }
            })
            // Two values (functions) ex: `abs(x)`
            .map_prefix(|op, rhs| match op.as_rule() {
                Rule::negation => MathExpression::Negation(Box::new(rhs)),
                Rule::abs => MathExpression::Abs(Box::new(rhs)),
                _ => unreachable!(),
            })
            .parse(pairs)
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
    let inner_rule = prim_rule
        .into_inner()
        .next()
        .expect("at least one sub rule");
    match &inner_rule.as_rule() {
        Rule::identifier => ArgType::Identifier(inner_rule.as_str().to_string()),
        Rule::value => {
            // either a number of string value:
            let inner_rule = inner_rule.into_inner().next().expect("at least one");
            match inner_rule.as_rule() {
                Rule::number => ArgType::Value({
                    // This is done to be sure that numbers have a decimal part for divsions
                    let float: f64 = inner_rule
                        .as_str()
                        .trim()
                        .parse::<f64>()
                        .expect("correct f64 value");

                    format!("{:?}", float)
                }),
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

#[cfg(test)]
mod tests {
    use crate::{
        database_handler::{
            ArgType, ClassSelection, ClassSelectionError, ClassType, ExtremalCounterQuery,
            SqlComparison, SqlCondition,
        },
        parser::query_parser::{ParsingError, QueryParser},
    };

    #[test]
    fn parse_comparison() {
        assert!(matches!(
            QueryParser::parse_condition("x > 1"),
            Ok(SqlCondition::Operation(SqlComparison::Greater(
                ArgType::Identifier(x),
                ArgType::Value(y)
            ))) if x == "x" && y == "1"
        ));
        assert!(matches!(
            QueryParser::parse_condition("(1 <= b2 )"),
            Ok(SqlCondition::Operation(SqlComparison::LessEqual(
                ArgType::Value(x),
                ArgType::Identifier(y)
            ))) if x == "1" && y == "b2"
        ));
    }

    #[test]
    fn parse_condition() {
        assert!(matches!(
            QueryParser::parse_condition("a > b and c = d"),
            Ok(SqlCondition::And(b1, b2))
                if *b1 == SqlCondition::Operation(SqlComparison::Greater(ArgType::Identifier("a".to_string()), ArgType::Identifier("b".to_string()))) &&
                    *b2 == SqlCondition::Operation(SqlComparison::Equal(ArgType::Identifier("c".to_string()), ArgType::Identifier("d".to_string())))
        ));
    }

    #[test]
    fn parse_condition_not() {
        assert!(matches!(
            QueryParser::parse_condition("not(a > b) or c = d"),
            Ok(SqlCondition::Or(b1, b2))
                if *b1 == SqlCondition::Not(Box::new(SqlCondition::Operation(SqlComparison::Greater(ArgType::Identifier("a".to_string()), ArgType::Identifier("b".to_string()))))) &&
                    *b2 == SqlCondition::Operation(SqlComparison::Equal(ArgType::Identifier("c".to_string()), ArgType::Identifier("d".to_string())))
        ));

        assert!(matches!(
            QueryParser::parse_condition("!inv or a"),
            Ok(SqlCondition::Or(b1, b2))
                if *b1 == SqlCondition::Operation(SqlComparison::Equal(ArgType::identifier("inv"), ArgType::value("0"))) &&
                    *b2 == SqlCondition::Operation(SqlComparison::Equal(ArgType::Identifier("a".to_string()), ArgType::Value("1".to_string())))
        ));
    }

    #[test]
    fn parse_condition_condition_chain() {
        assert!(matches!(
            QueryParser::parse_condition("a > b or c = d and 0 != 5"),
            Ok(SqlCondition::Or(b1, b2))
                if *b1 == SqlCondition::Operation(SqlComparison::Greater(ArgType::Identifier("a".to_string()), ArgType::Identifier("b".to_string()))) &&
                    *b2 == SqlCondition::And(Box::new(SqlCondition::Operation(SqlComparison::Equal(ArgType::Identifier("c".to_string()), ArgType::Identifier("d".to_string())))),
                        Box::new(SqlCondition::Operation(SqlComparison::NotEqual(ArgType::Value("0".to_string()), ArgType::Value("5".to_string())))))
        ));
    }

    #[test]
    fn parse_condition_condition_parenthesis() {
        assert!(matches!(
            QueryParser::parse_condition("(a > b or c = d) and 0 != 5"),
            Ok(SqlCondition::And(b1, b2))
                if *b2 == SqlCondition::Operation(SqlComparison::NotEqual(ArgType::Value("0".to_string()), ArgType::Value("5".to_string()))) &&
                    *b1 == SqlCondition::Or(Box::new(SqlCondition::Operation(SqlComparison::Greater(ArgType::Identifier("a".to_string()), ArgType::Identifier("b".to_string())))),
                Box::new(SqlCondition::Operation(SqlComparison::Equal(ArgType::Identifier("c".to_string()), ArgType::Identifier("d".to_string())))))
        ));
    }

    #[test]
    fn parse_extremal_query() {
        assert!(matches!(
            QueryParser::parse_extremal_query("min(p_gn: m,n), d_nm >= 3"),
            Ok(
                (selection, additional_condition)
            )
            if selection == ClassSelection::new(ClassType::Min, "p_gn", vec!["m", "n"]).expect("correct") && additional_condition == Some(SqlCondition::Operation(SqlComparison::GreaterEqual(
            ArgType::Identifier("d_nm".to_string()),
            ArgType::Value("3".to_string())),
        ))));

        assert!(matches!(
        QueryParser::parse_extremal_query("min(p_gn)"),
        Ok(
            (selection, additional_condition)
        )
        if selection == ClassSelection::new(ClassType::Min, "p_gn", Vec::<String>::new()).expect("correct") && additional_condition.is_none()
        ));

        assert!(matches!(
            QueryParser::parse_extremal_query("min(p_gn), d >"),
            Err(ParsingError::ParseError {
                col_pos: _,
                input: _,
                missing_token: _
            })
        ));

        assert!(matches!(
            QueryParser::parse_extremal_query("min(p_gn: p_gn), d > 4"),
            Err(ParsingError::ClassSelectionError(_, ClassSelectionError::CombineWithItself(val))) if val == "p_gn"
        ));

        assert!(matches!(
            QueryParser::parse_extremal_query("min(p_gn: g, d, g), d > 4"),
            Err(ParsingError::ClassSelectionError(_, ClassSelectionError::DuplicateInv(val))) if val == "g"
        ));
    }

    #[test]
    fn parse_conj_query() {
        assert!(matches!(
            QueryParser::parse_extr_conj_query("min(p_gn: m,n), d_nm >= 3 => conj1"),
            Ok(
                ExtremalCounterQuery{additional_condition, selection, conjecture_to_disprove}
            )
            if selection == ClassSelection::new(ClassType::Min, "p_gn", vec!["m", "n"]).expect("correct") && additional_condition == Some(SqlCondition::Operation(SqlComparison::GreaterEqual(
            ArgType::Identifier("d_nm".to_string()),
            ArgType::Value("3".to_string())),
        )) && conjecture_to_disprove == SqlCondition::Operation(SqlComparison::Equal(ArgType::Identifier("conj1".to_string()), ArgType::Value("1".to_string())))));

        assert!(matches!(
            QueryParser::parse_extr_conj_query("min(p_gn) => conj1 = 1"),
            Ok(
                ExtremalCounterQuery{additional_condition, selection, conjecture_to_disprove}
            )
            if selection == ClassSelection::new(ClassType::Min, "p_gn", Vec::<String>::new()).expect("correct") && additional_condition.is_none() 
            && conjecture_to_disprove == SqlCondition::Operation(SqlComparison::Equal(ArgType::Identifier("conj1".to_string()), ArgType::Value("1".to_string())))));

        assert!(matches!(
            QueryParser::parse_extr_conj_query("min(p_gn: p_gn) => conj1"),
            Err(ParsingError::ClassSelectionError(_, ClassSelectionError::CombineWithItself(val))) if val == "p_gn"
        ));

        assert!(matches!(
            QueryParser::parse_extr_conj_query("min(p_gn: g, d, g) => conj1 = 1"),
            Err(ParsingError::ClassSelectionError(_, ClassSelectionError::DuplicateInv(val))) if val == "g"
        ));
    }

    #[test]
    fn parse_query() {
        let sql_cond = SqlCondition::Operation(SqlComparison::Equal(
            ArgType::Identifier("x".to_string()),
            ArgType::Value("1".to_string()),
        ));

        let extremal = (
            ClassSelection::new(ClassType::Min, "p_gn", vec!["g", "d"]).expect("correct"),
            Some(sql_cond.clone()),
        );

        let conjecture = ExtremalCounterQuery {
            selection: extremal.0.clone(),
            additional_condition: extremal.1.clone(),
            conjecture_to_disprove: SqlCondition::not(SqlComparison::Equal(
                ArgType::Value("1".to_string()),
                ArgType::Identifier("p".to_string()),
            )),
        };

        assert!(
            matches!(QueryParser::parse_query("x"), Ok(crate::parser::query_parser::ParsedQuery::Condition(x)) if x == sql_cond )
        );

        assert!(
            matches!(QueryParser::parse_query("min(p_gn: g, d), x = 1"), Ok(crate::parser::query_parser::ParsedQuery::Extremal(val)) if val == extremal )
        );

        assert!(
            matches!(QueryParser::parse_query("min(p_gn: g, d), x = 1 => !(1 = p)"), Ok(crate::parser::query_parser::ParsedQuery::ExtremalCounter(val)) if val == conjecture )
        )
    }

    #[test]
    fn parse_expression() {
        let parse_expr = QueryParser::parse_expression("2 ** 2 + 1 // 2").expect("correct");
        println!("{parse_expr}")
    }
}
