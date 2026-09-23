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
            if left == Condition::Comparison(a).into() && *right == QueryStatement::extremal_condition(b.to_condition())));
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

    let expected = QueryStatement::if_then(
        a.clone().to_condition(),
        QueryStatement::if_then(b.clone().to_condition(), c.clone()),
    );

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
        QueryParser::parse_condition("not a > b or c = d", None),
        Ok(Condition::Or(b1, b2))
            if *b1 == Condition::Not(Box::new(Comparison::Greater(ParsedArgType::invariant("a").into(), ParsedArgType::invariant("b").into()).into())) &&
                *b2 == Condition::Comparison(Comparison::Equal(ParsedArgType::invariant("c").into(), ParsedArgType::invariant("d").into(), None))
    ));

    assert_eq!(
        QueryParser::parse_condition("not a > b or c = d", None).expect("correct"),
        QueryParser::parse_condition("not (a > b) or c = d", None).expect("correct")
    );
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

#[test]
fn parse_extremal_query() {
    let tmp = QueryParser::parse_extremal("max(eci(comp(G)); m(comp), n*2**2)").expect("correct");
    // TODO: Add unit test
    println!("{tmp}")
}


#[test]
fn parse_extremal_if_then_query() {
    let tmp = QueryParser::parse_query("x > 1 -> min(x) -> max(p;n,m) -> y and z", None).expect("correct");
    // TODO: Add unit test
    println!("{tmp:?}")
}

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
