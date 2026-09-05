use gquest_core::parser::{
    parsed_expression::{
        ArithmOp, Comparison, Condition, MathExpression, ParsedArgType, QueryStatement,
    },
    query_parser::QueryParser,
};

#[test]
fn parse_comparison() {
    assert!(matches!(
        QueryParser::parse_condition("x > 1", None),
        Ok(Condition::Comparison(Comparison::Greater(
            MathExpression::Primitif(x),
            MathExpression::Primitif(y)
        ))) if x == ParsedArgType::invariant("x") && y == ParsedArgType::prim_numeric(1.)
    ));
    assert!(matches!(
        QueryParser::parse_condition("(1 <= b2 )", None),
        Ok(Condition::Comparison(Comparison::LessEqual(
            MathExpression::Primitif(x),
            MathExpression::Primitif(y),
            None
        ))) if x == ParsedArgType::prim_numeric(1.) && y == ParsedArgType::invariant("b2")
    ));
}

#[test]
fn parse_condition() {
    let a = ParsedArgType::invariant("a");
    let b = ParsedArgType::invariant("b");
    let c = ParsedArgType::invariant("c");
    let d = ParsedArgType::invariant("d");
    assert!(matches!(
        QueryParser::parse_condition("a > b and c = d", None),
        Ok(Condition::And(b1, b2))
            if *b1 == Condition::Comparison(Comparison::Greater(a.into(), b.into())) &&
                *b2 == Condition::Comparison(Comparison::Equal(c.into(), d.into(), None))
    ));
}

#[test]
fn parse_query_if_then() {
    let a = Comparison::Equal(
        ParsedArgType::invariant("a").into(),
        ParsedArgType::prim_numeric(1.).into(),
        None,
    );
    let b = Comparison::Equal(
        ParsedArgType::invariant("b").into(),
        ParsedArgType::prim_numeric(1.).into(),
        None,
    );

    assert!(matches!(
        QueryParser::parse_query("a -> b", None),
        Ok(QueryStatement::IfThen(left, right))
            if left == Condition::Comparison(a) && *right == QueryStatement::condition(b)));
}

#[test]
fn parse_query_if_then_chained() {
    let a = Comparison::Equal(
        ParsedArgType::invariant("a").into(),
        ParsedArgType::prim_numeric(1.).into(),
        None,
    );
    let b = Comparison::Equal(
        ParsedArgType::invariant("b").into(),
        ParsedArgType::prim_numeric(1.).into(),
        None,
    );
    let c: Condition = Comparison::Equal(
        ParsedArgType::invariant("c").into(),
        ParsedArgType::prim_numeric(1.).into(),
        None,
    )
    .into();
    let expected =
        QueryStatement::if_then(a.clone(), QueryStatement::if_then(b.clone(), c.clone()));

    assert!(matches!(
        QueryParser::parse_query("a -> b -> c", None),
        Ok(received)
            if received == expected));
}

#[test]
fn parse_condition_xor() {
    let a = ParsedArgType::invariant("a");
    let b = ParsedArgType::invariant("b");
    let one = ParsedArgType::prim_numeric(1.);
    let a_true = Comparison::Equal(a.into(), one.clone().into(), None);
    let b_true = Comparison::Equal(b.into(), one.clone().into(), None);

    let cond = Condition::and(
        Condition::or(a_true.clone(), b_true.clone()),
        Condition::not(Condition::and(a_true.clone(), b_true.clone())),
    );

    assert!(matches!(
        QueryParser::parse_condition("a = 1 xor b = 1", None),
        Ok(cond1)
        if cond1 == cond
    ))
}

#[test]
fn parse_condition_equivalence() {
    let a = ParsedArgType::invariant("a");
    let b = ParsedArgType::invariant("b");
    let one = ParsedArgType::prim_numeric(1.);
    let a_true = Comparison::Equal(a.into(), one.clone().into(), None);
    let b_true = Comparison::Equal(b.into(), one.clone().into(), None);

    let cond = Condition::equivalence(a_true, b_true);

    assert!(matches!(
        QueryParser::parse_condition("a <==> b", None),
        Ok(cond1)
        if cond1 == cond
    ))
}

#[test]
fn parse_condition_not() {
    assert!(matches!(
        QueryParser::parse_condition("not(a > b) or c = d", None),
        Ok(Condition::Or(b1, b2))
            if *b1 == Condition::Not(Box::new(Condition::Comparison(Comparison::Greater(ParsedArgType::invariant("a").into(), ParsedArgType::invariant("b").into())))) &&
                *b2 == Condition::Comparison(Comparison::Equal(ParsedArgType::invariant("c").into(), ParsedArgType::invariant("d").into(), None))
    ));

    assert!(matches!(
        QueryParser::parse_condition("!inv or a", None),
        Ok(Condition::Or(b1, b2))
            if *b1 == Condition::Comparison(Comparison::Equal(ParsedArgType::invariant("inv").into(), ParsedArgType::prim_numeric(0.).into(), None)) &&
                *b2 == Condition::Comparison(Comparison::Equal(ParsedArgType::invariant("a").into(), ParsedArgType::prim_numeric(1.).into(), None))
    ));
}

#[test]
fn parse_condition_condition_chain() {
    assert!(matches!(
        QueryParser::parse_condition("a > b or c = d and 0 != 5", None),
        Ok(Condition::Or(b1, b2))
            if *b1 == Condition::Comparison(Comparison::Greater(ParsedArgType::invariant("a").into(), ParsedArgType::invariant("b").into())) &&
                *b2 == Condition::And(Box::new(Condition::Comparison(Comparison::Equal(ParsedArgType::invariant("c").into(), ParsedArgType::invariant("d").into(), None))),
                    Box::new(Condition::Comparison(Comparison::NotEqual(ParsedArgType::prim_numeric(0.).into(), ParsedArgType::prim_numeric(5.).into(), None))))
    ));
}

#[test]
fn parse_condition_condition_parenthesis() {
    assert!(matches!(
        QueryParser::parse_condition("(a > b or c = d) and 0 != 5", None),
        Ok(Condition::And(b1, b2))
            if *b2 == Condition::Comparison(Comparison::NotEqual(ParsedArgType::prim_numeric(0.).into(), ParsedArgType::prim_numeric(5.).into(), None)) &&
                *b1 == Condition::Or(Box::new(Condition::Comparison(Comparison::Greater(ParsedArgType::invariant("a").into(), ParsedArgType::invariant("b").into()))),
            Box::new(Condition::Comparison(Comparison::Equal(ParsedArgType::invariant("c").into(), ParsedArgType::invariant("d").into(), None))))
    ));
}

// #[test]
// fn parse_extremal_query() {
//     assert!(matches!(
//         QueryParser::parse_extremal_condition("min(p_gn: m,n) and d_nm >= 3", None),
//         Ok(
//             ParsedCondition::ExtremalCondition(selection, additional_condition)
//         )
//         if selection == ClassSelection::new(ClassType::Min, "p_gn", vec!["m", "n"]).expect("correct") && additional_condition == Some(SqlCondition::Operation(SqlComparison::GreaterEqual(
//         ArgType::identifier("d_nm").into(),
//         ArgType::value("3.0").into(), None),
//     ))));

//     assert!(matches!(
//     QueryParser::parse_extremal_condition("min(p_gn)", None),
//     Ok(
//         ParsedCondition::ExtremalCondition(selection, None)
//     )
//     if selection == ClassSelection::new(ClassType::Min, "p_gn", Vec::<String>::new()).expect("correct")
//     ));

//     assert!(matches!(
//         QueryParser::parse_extremal_condition("min(p_gn) and d >", None),
//         Err(ParsingError::ParseError {
//             col_pos: _,
//             input: _,
//             missing_token: _
//         })
//     ));

//     assert!(matches!(
//         QueryParser::parse_extremal_condition("min(p_gn: p_gn) and d > 4",None),
//         Err(ParsingError::ClassSelectionError(_, ClassSelectionError::CombineWithItself(val))) if val == "p_gn"
//     ));

//     assert!(matches!(
//         QueryParser::parse_extremal_condition("min(p_gn: g, d, g) and d > 4",None),
//         Err(ParsingError::ClassSelectionError(_, ClassSelectionError::DuplicateInv(val))) if val == "g"
//     ));
// }

// #[test]
// fn parse_if_else_query_extremal() {
//     assert!(matches!(
//             QueryParser::parse_query("min(p_gn: m,n) and d_nm >= 3 -> conj1",None),
//             Ok(
//                 ParsedQuery::IfElse(ParsedCondition::ExtremalCondition(selection, add_cond), cond)
//             )
//             if selection == ClassSelection::new(ClassType::Min, "p_gn", vec!["m", "n"]).expect("correct") && add_cond == Some(SqlCondition::Operation(SqlComparison::GreaterEqual(
//             ArgType::identifier("d_nm").into(),
//             ArgType::value("3.0").into(), None),
//         )) && cond == SqlCondition::Operation(SqlComparison::Equal(ArgType::identifier("conj1").into(), ArgType::value("1").into(), None
//     ))));

//     assert!(matches!(
//         QueryParser::parse_query("min(p_gn) -> conj1 = 1", None),
//         Ok(
//                 ParsedQuery::IfElse(ParsedCondition::ExtremalCondition(selection, None), cond)
//             )
//         if selection == ClassSelection::new(ClassType::Min, "p_gn", Vec::<String>::new()).expect("correct")
//         && cond == SqlCondition::Operation(SqlComparison::Equal(ArgType::identifier("conj1").into(), ArgType::value("1.0").into(), None))
//     ));

//     assert!(matches!(
//         QueryParser::parse_query("min(p_gn: p_gn) -> conj1", None),
//         Err(ParsingError::ClassSelectionError(_, ClassSelectionError::CombineWithItself(val))) if val == "p_gn"
//     ));

//     assert!(matches!(
//         QueryParser::parse_query("min(p_gn: g, d, g) -> conj1 = 1", None),
//         Err(ParsingError::ClassSelectionError(_, ClassSelectionError::DuplicateInv(val))) if val == "g"
//     ));
// }

#[test]
fn parse_bin_expression() {
    let parse_expr = QueryParser::parse_expression("2 ** 2 + a // (2- 1*5)").expect("correct");

    let correct_expr = bin_operation(
        bin_operation(
            ParsedArgType::prim_numeric(2.),
            ArithmOp::Power,
            ParsedArgType::prim_numeric(2.),
        ),
        ArithmOp::Add,
        floor(bin_operation(
            ParsedArgType::invariant("a"),
            ArithmOp::Divide,
            bin_operation(
                ParsedArgType::prim_numeric(2.),
                ArithmOp::Subtract,
                bin_operation(
                    ParsedArgType::prim_numeric(1.),
                    ArithmOp::Multiply,
                    ParsedArgType::prim_numeric(5.),
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
            ParsedArgType::prim_numeric(1.),
            ArithmOp::Divide,
            ParsedArgType::prim_numeric(1.),
        )),
        ArithmOp::Modulo,
        floor(ParsedArgType::invariant("a")),
    );

    assert_eq!(parse_expr, correct_expr);

    let parse_expr = QueryParser::parse_expression("sqrt(a) / ceil(1 + 1)").expect("correct");

    let correct_expr = bin_operation(
        sqrt(ParsedArgType::invariant("a")),
        ArithmOp::Divide,
        ceil(bin_operation(
            ParsedArgType::prim_numeric(1.),
            ArithmOp::Add,
            ParsedArgType::prim_numeric(1.),
        )),
    );

    assert_eq!(parse_expr, correct_expr);
}

#[test]
fn parse_negation_expression() {
    let parse_expr = QueryParser::parse_expression("-(1 + a)").expect("correct");

    let correct_expr = negation(bin_operation(
        ParsedArgType::prim_numeric(1.),
        ArithmOp::Add,
        ParsedArgType::invariant("a"),
    ));

    assert_eq!(parse_expr, correct_expr);

    let parse_expr = QueryParser::parse_expression("-a").expect("correct");

    let correct_expr = negation(ParsedArgType::invariant("a"));

    assert_eq!(parse_expr, correct_expr);
}

#[test]
fn parse_expression_comparison() {
    assert!(matches!(
        QueryParser::parse_condition("n + 1 > 0", None),
        Ok(Condition::Comparison(Comparison::Greater(a, b)))
        if a == bin_operation(ParsedArgType::invariant("n"), ArithmOp::Add, ParsedArgType::prim_numeric(1.))
        && b == ParsedArgType::prim_numeric(0.).into()
    ));

    assert!(matches!(
        QueryParser::parse_condition("floor(n) = 0 % (2 * 1)", None),
        Ok(Condition::Comparison(Comparison::Equal(a, b, None)))
        if
        a == floor(ParsedArgType::invariant("n"))
        &&
        b == bin_operation(ParsedArgType::prim_numeric(0.), ArithmOp::Modulo, bin_operation(ParsedArgType::prim_numeric(2.), ArithmOp::Multiply, ParsedArgType::prim_numeric(1.)))
    ));

    assert!(matches!(
        QueryParser::parse_condition("a = n ** 2", None),
        Ok(Condition::Comparison(Comparison::Equal(a, b, None)))
        if
        a == ParsedArgType::invariant("a").into()
        &&
        b == bin_operation(ParsedArgType::invariant("n"), ArithmOp::Power, ParsedArgType::prim_numeric(2.))
    ));
}

#[test]
fn parse_function() {
    assert!(matches!(
        QueryParser::parse_condition("fn(G, 12) > 0", None),
        Ok(Condition::Comparison(Comparison::Greater(a, b)))
        if a == ParsedArgType::function("fn", ParsedArgType::Dataset, vec![ParsedArgType::prim_numeric(12.0)]).into()
        && b == ParsedArgType::prim_numeric(0.0).into()
    ));

    assert!(matches!(
        QueryParser::parse_condition("fn(G) > 0", None),
        Ok(Condition::Comparison(Comparison::Greater(a, b)))
        if a == ParsedArgType::function("fn", ParsedArgType::Dataset, Vec::<ParsedArgType>::new()).into()
        && b == ParsedArgType::prim_numeric(0.0).into()
    ));

    assert!(matches!(
        QueryParser::parse_condition("fn > 0", None),
        Ok(Condition::Comparison(Comparison::Greater(a, b)))
        if a == ParsedArgType::function("fn", ParsedArgType::Dataset, Vec::<ParsedArgType>::new()).into()
        && b == ParsedArgType::prim_numeric(0.0).into()
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
