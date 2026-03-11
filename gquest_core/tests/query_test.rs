use gquest_core::{
    database_handler::{
        ArgType, ArithmOp, ClassSelection, ClassSelectionError, ClassType, MathExpression,
        SqlComparison, SqlCondition,
    },
    parser::query_parser::{ParsedCondition, ParsedQuery2, ParsingError, QueryParser},
};

#[test]
fn parse_comparison() {
    assert!(matches!(
        QueryParser::parse_condition("x > 1", None),
        Ok(SqlCondition::Operation(SqlComparison::Greater(
            MathExpression::Primitif(ArgType::Identifier(x)),
            MathExpression::Primitif(ArgType::Value(y))
        ))) if x == "x" && y == "1.0"
    ));
    assert!(matches!(
        QueryParser::parse_condition("(1 <= b2 )", None),
        Ok(SqlCondition::Operation(SqlComparison::LessEqual(
            MathExpression::Primitif(ArgType::Value(x)),
            MathExpression::Primitif(ArgType::Identifier(y)),
            None
        ))) if x == "1.0" && y == "b2"
    ));
}

#[test]
fn parse_condition() {
    assert!(matches!(
        QueryParser::parse_condition("a > b and c = d", None),
        Ok(SqlCondition::And(b1, b2))
            if *b1 == SqlCondition::Operation(SqlComparison::Greater(ArgType::Identifier("a".to_string()).into(), ArgType::Identifier("b".to_string()).into())) &&
                *b2 == SqlCondition::Operation(SqlComparison::Equal(ArgType::Identifier("c".to_string()).into(), ArgType::Identifier("d".to_string()).into(), None))
    ));
}

#[test]
fn parse_condition_xor() {
    let a = ArgType::identifier("a");
    let b = ArgType::identifier("b");
    let one = ArgType::value("1.0");
    let a_true = SqlComparison::Equal(a.into(), one.clone().into(), None);
    let b_true = SqlComparison::Equal(b.into(), one.clone().into(), None);

    let cond = SqlCondition::and(
        SqlCondition::or(a_true.clone(), b_true.clone()),
        SqlCondition::not(SqlCondition::and(a_true.clone(), b_true.clone())),
    );

    assert!(matches!(
        QueryParser::parse_condition("a = 1 xor b = 1", None),
        Ok(cond1)
        if cond1 == cond
    ))
}

#[test]
fn parse_condition_not() {
    assert!(matches!(
        QueryParser::parse_condition("not(a > b) or c = d", None),
        Ok(SqlCondition::Or(b1, b2))
            if *b1 == SqlCondition::Not(Box::new(SqlCondition::Operation(SqlComparison::Greater(ArgType::Identifier("a".to_string()).into(), ArgType::Identifier("b".to_string()).into())))) &&
                *b2 == SqlCondition::Operation(SqlComparison::Equal(ArgType::Identifier("c".to_string()).into(), ArgType::Identifier("d".to_string()).into(), None))
    ));

    assert!(matches!(
        QueryParser::parse_condition("!inv or a", None),
        Ok(SqlCondition::Or(b1, b2))
            if *b1 == SqlCondition::Operation(SqlComparison::Equal(ArgType::identifier("inv").into(), ArgType::value("0").into(), None)) &&
                *b2 == SqlCondition::Operation(SqlComparison::Equal(ArgType::identifier("a").into(), ArgType::value("1").into(), None))
    ));
}

#[test]
fn parse_condition_condition_chain() {
    assert!(matches!(
        QueryParser::parse_condition("a > b or c = d and 0 != 5", None),
        Ok(SqlCondition::Or(b1, b2))
            if *b1 == SqlCondition::Operation(SqlComparison::Greater(ArgType::identifier("a").into(), ArgType::identifier("b").into())) &&
                *b2 == SqlCondition::And(Box::new(SqlCondition::Operation(SqlComparison::Equal(ArgType::identifier("c").into(), ArgType::identifier("d").into(), None))),
                    Box::new(SqlCondition::Operation(SqlComparison::NotEqual(ArgType::value("0.0").into(), ArgType::value("5.0").into(), None))))
    ));
}

#[test]
fn parse_condition_condition_parenthesis() {
    assert!(matches!(
        QueryParser::parse_condition("(a > b or c = d) and 0 != 5", None),
        Ok(SqlCondition::And(b1, b2))
            if *b2 == SqlCondition::Operation(SqlComparison::NotEqual(ArgType::value("0.0").into(), ArgType::value("5.0").into(), None)) &&
                *b1 == SqlCondition::Or(Box::new(SqlCondition::Operation(SqlComparison::Greater(ArgType::identifier("a").into(), ArgType::identifier("b").into()))),
            Box::new(SqlCondition::Operation(SqlComparison::Equal(ArgType::identifier("c").into(), ArgType::identifier("d").into(), None))))
    ));
}

#[test]
fn parse_extremal_query() {
    assert!(matches!(
        QueryParser::parse_extremal_condition("min(p_gn: m,n) and d_nm >= 3", None),
        Ok(
            ParsedCondition::ExtremalCondition(selection, additional_condition)
        )
        if selection == ClassSelection::new(ClassType::Min, "p_gn", vec!["m", "n"]).expect("correct") && additional_condition == Some(SqlCondition::Operation(SqlComparison::GreaterEqual(
        ArgType::identifier("d_nm").into(),
        ArgType::value("3.0").into(), None),
    ))));

    assert!(matches!(
    QueryParser::parse_extremal_condition("min(p_gn)", None),
    Ok(
        ParsedCondition::ExtremalCondition(selection, None)
    )
    if selection == ClassSelection::new(ClassType::Min, "p_gn", Vec::<String>::new()).expect("correct")
    ));

    assert!(matches!(
        QueryParser::parse_extremal_condition("min(p_gn) and d >", None),
        Err(ParsingError::ParseError {
            col_pos: _,
            input: _,
            missing_token: _
        })
    ));

    assert!(matches!(
        QueryParser::parse_extremal_condition("min(p_gn: p_gn) and d > 4",None),
        Err(ParsingError::ClassSelectionError(_, ClassSelectionError::CombineWithItself(val))) if val == "p_gn"
    ));

    assert!(matches!(
        QueryParser::parse_extremal_condition("min(p_gn: g, d, g) and d > 4",None),
        Err(ParsingError::ClassSelectionError(_, ClassSelectionError::DuplicateInv(val))) if val == "g"
    ));
}

#[test]
fn parse_if_else_query_extremal() {
    assert!(matches!(
            QueryParser::parse_query("min(p_gn: m,n) and d_nm >= 3 -> conj1",None),
            Ok(
                ParsedQuery2::IfElse(ParsedCondition::ExtremalCondition(selection, add_cond), cond)
            )
            if selection == ClassSelection::new(ClassType::Min, "p_gn", vec!["m", "n"]).expect("correct") && add_cond == Some(SqlCondition::Operation(SqlComparison::GreaterEqual(
            ArgType::identifier("d_nm").into(),
            ArgType::value("3.0").into(), None),
        )) && cond == SqlCondition::Operation(SqlComparison::Equal(ArgType::identifier("conj1").into(), ArgType::value("1").into(), None
    ))));

    assert!(matches!(
        QueryParser::parse_query("min(p_gn) -> conj1 = 1", None),
        Ok(
                ParsedQuery2::IfElse(ParsedCondition::ExtremalCondition(selection, None), cond)
            )
        if selection == ClassSelection::new(ClassType::Min, "p_gn", Vec::<String>::new()).expect("correct")
        && cond == SqlCondition::Operation(SqlComparison::Equal(ArgType::identifier("conj1").into(), ArgType::value("1.0").into(), None))
    ));

    assert!(matches!(
        QueryParser::parse_query("min(p_gn: p_gn) -> conj1", None),
        Err(ParsingError::ClassSelectionError(_, ClassSelectionError::CombineWithItself(val))) if val == "p_gn"
    ));

    assert!(matches!(
        QueryParser::parse_query("min(p_gn: g, d, g) -> conj1 = 1", None),
        Err(ParsingError::ClassSelectionError(_, ClassSelectionError::DuplicateInv(val))) if val == "g"
    ));
}

#[test]
fn parse_query() {
    let sql_cond = SqlCondition::Operation(SqlComparison::Equal(
        ArgType::Identifier("x".to_string()).into(),
        ArgType::Value("1.0".to_string()).into(),
        None,
    ));

    let extremal = ClassSelection::new(ClassType::Min, "p_gn", vec!["g", "d"]).expect("correct");
    let sql_cond_2 = SqlCondition::not(SqlComparison::Equal(
        ArgType::Value("1.0".to_string()).into(),
        ArgType::Identifier("p".to_string()).into(),
        None,
    ));

    assert!(
        matches!(QueryParser::parse_query("x = 1", None), Ok(ParsedQuery2::Condition(ParsedCondition::Condition(x))) if x == sql_cond )
    );

    assert!(
        matches!(QueryParser::parse_query("min(p_gn: g, d) and x = 1", None), Ok(ParsedQuery2::Condition(ParsedCondition::ExtremalCondition(selection, Some(cond)))) if selection == extremal && cond == sql_cond)
    );

    assert!(
        matches!(QueryParser::parse_query("min(p_gn: g, d) and x = 1 -> !(1 = p)", None), Ok(ParsedQuery2::IfElse(ParsedCondition::ExtremalCondition(selection, Some(add_cond)), cond)) 
            if selection == extremal && add_cond == sql_cond && cond == sql_cond_2 )
    );

    assert!(
        matches!(QueryParser::parse_query("x = 1 -> !(1 = p)", None), Ok(ParsedQuery2::IfElse(ParsedCondition::Condition(cond_1), cond_2)) 
            if cond_1 == sql_cond && cond_2 == sql_cond_2 )
    )
}

#[test]
fn parse_bin_expression() {
    let parse_expr = QueryParser::parse_expression("2 ** 2 + a // (2- 1*5)").expect("correct");

    let correct_expr = bin_operation(
        bin_operation(
            ArgType::value("2.0"),
            ArithmOp::Power,
            ArgType::value("2.0"),
        ),
        ArithmOp::Add,
        floor(bin_operation(
            ArgType::identifier("a"),
            ArithmOp::Divide,
            bin_operation(
                ArgType::value("2.0"),
                ArithmOp::Subtract,
                bin_operation(
                    ArgType::value("1.0"),
                    ArithmOp::Multiply,
                    ArgType::value("5.0"),
                ),
            ),
        )),
    );
    assert_eq!(parse_expr, correct_expr)
}

#[test]
fn parse_unary_expression() {
    let parse_expr = QueryParser::parse_expression("abs(1/1) % floor(a)").expect("correct");

    let correct_expr = bin_operation(
        abs(bin_operation(
            ArgType::value("1.0"),
            ArithmOp::Divide,
            ArgType::value("1.0"),
        )),
        ArithmOp::Modulo,
        floor(ArgType::identifier("a")),
    );

    assert_eq!(parse_expr, correct_expr);

    let parse_expr = QueryParser::parse_expression("sqrt(a) / ceil(1 + 1)").expect("correct");

    let correct_expr = bin_operation(
        sqrt(ArgType::identifier("a")),
        ArithmOp::Divide,
        ceil(bin_operation(
            ArgType::value("1.0"),
            ArithmOp::Add,
            ArgType::value("1.0"),
        )),
    );

    assert_eq!(parse_expr, correct_expr);
}

#[test]
fn parse_negation_expression() {
    let parse_expr = QueryParser::parse_expression("-(1 + a)").expect("correct");

    let correct_expr = negation(bin_operation(
        ArgType::value("1.0"),
        ArithmOp::Add,
        ArgType::identifier("a"),
    ));

    assert_eq!(parse_expr, correct_expr);

    let parse_expr = QueryParser::parse_expression("-a").expect("correct");

    let correct_expr = negation(ArgType::identifier("a"));

    assert_eq!(parse_expr, correct_expr);
}

#[test]
fn parse_expression_comparison() {
    assert!(matches!(
        QueryParser::parse_condition("n + 1 > 0", None),
        Ok(SqlCondition::Operation(SqlComparison::Greater(a, b)))
        if a == bin_operation(ArgType::identifier("n"), ArithmOp::Add, ArgType::value("1.0"))
        && b == ArgType::value("0.0").into()
    ));

    assert!(matches!(
        QueryParser::parse_condition("floor(n) = 0 % (2 * 1)", None),
        Ok(SqlCondition::Operation(SqlComparison::Equal(a, b, None)))
        if
        a == floor(ArgType::identifier("n"))
        &&
        b == bin_operation(ArgType::value("0.0"), ArithmOp::Modulo, bin_operation(ArgType::value("2.0"), ArithmOp::Multiply, ArgType::value("1.0")))
    ));

    assert!(matches!(
        QueryParser::parse_condition("a = n ** 2", None),
        Ok(SqlCondition::Operation(SqlComparison::Equal(a, b, None)))
        if
        a == ArgType::identifier("a").into()
        &&
        b == bin_operation(ArgType::identifier("n"), ArithmOp::Power, ArgType::value("2.0"))
    ));
}

// Functions used to write less code :
fn floor(expr: impl Into<MathExpression>) -> MathExpression {
    MathExpression::Floor(Box::new(expr.into()))
}
fn ceil(expr: impl Into<MathExpression>) -> MathExpression {
    MathExpression::Ceil(Box::new(expr.into()))
}
fn abs(expr: impl Into<MathExpression>) -> MathExpression {
    MathExpression::Abs(Box::new(expr.into()))
}
fn negation(expr: impl Into<MathExpression>) -> MathExpression {
    MathExpression::Negation(Box::new(expr.into()))
}
fn sqrt(expr: impl Into<MathExpression>) -> MathExpression {
    MathExpression::Sqrt(Box::new(expr.into()))
}
fn bin_operation(
    left: impl Into<MathExpression>,
    op: ArithmOp,
    right: impl Into<MathExpression>,
) -> MathExpression {
    MathExpression::bin_operation(left, op, right)
}
