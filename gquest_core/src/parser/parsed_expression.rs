use std::fmt::Display;

use crate::EqF64;

#[derive(Debug, PartialEq)]
pub enum QueryStatement {
    Condition(Condition),
    IfThen(Condition, Box<QueryStatement>),
}

impl From<Condition> for QueryStatement {
    fn from(val: Condition) -> Self {
        QueryStatement::Condition(val)
    }
}

impl QueryStatement {
    pub fn condition(condition: impl Into<Condition>) -> Self {
        Self::Condition(condition.into())
    }

    pub fn if_then(
        condition: impl Into<Condition>,
        query_statement: impl Into<QueryStatement>,
    ) -> Self {
        Self::IfThen(condition.into(), Box::new(query_statement.into()))
    }

    pub fn to_counter(&mut self) {
        let mut statement = self;
        loop {
            match statement {
                QueryStatement::Condition(condition) => {
                    *condition = Condition::not(condition.clone());
                    break;
                }
                QueryStatement::IfThen(_, query_statement) => {
                    statement = query_statement;
                }
            }
        }
    }
}

/// Represents a **parsed** condition. Not yet typed checked.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition<P = ParsedArgType> {
    /// A simple comparison
    Comparison(Comparison<P>),
    /// `x` or `y`
    Or(Box<Condition<P>>, Box<Condition<P>>),
    /// `x` and `y`
    And(Box<Condition<P>>, Box<Condition<P>>),
    /// not `x`
    Not(Box<Condition<P>>),
}

impl<P: Display> Display for Condition<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parenthesis_fn = |val: &Condition<P>| -> String {
            if val.is_condition() {
                val.to_string()
            } else {
                format!("({val})")
            }
        };

        write!(
            f,
            "{}",
            match self {
                Condition::Comparison(comparison) => comparison.to_string(),
                Condition::Or(left_cond, right_cond) => format!(
                    "{} or {}",
                    parenthesis_fn(left_cond),
                    parenthesis_fn(right_cond)
                ),
                Condition::And(left_cond, right_cond) => format!(
                    "{} and {}",
                    parenthesis_fn(left_cond),
                    parenthesis_fn(right_cond)
                ),
                Condition::Not(cond) => format!("not {}", parenthesis_fn(cond)),
            }
        )
    }
}

impl<P> Condition<P>
where
    P: Clone,
{
    /// Simplifies the creation of the [`Condition::And`] enum.
    pub fn and(cond_1: impl Into<Condition<P>>, cond_2: impl Into<Condition<P>>) -> Self {
        Self::And(Box::new(cond_1.into()), Box::new(cond_2.into()))
    }

    /// Simplifies the creation of the [`Condition::Or`] enum.
    pub fn or(cond_1: impl Into<Condition<P>>, cond_2: impl Into<Condition<P>>) -> Self {
        Self::Or(Box::new(cond_1.into()), Box::new(cond_2.into()))
    }

    /// Simplifies the creation of the [`Condition::Not`] enum.
    pub fn not(cond: impl Into<Condition<P>>) -> Self {
        Self::Not(Box::new(cond.into()))
    }

    /// Creates an equivalente condition to the `xor` operator.
    /// `x xor y` iff `(x or y) and not(x and y)`
    pub fn xor(cond_1: impl Into<Condition<P>>, cond_2: impl Into<Condition<P>>) -> Self {
        let cond_1 = cond_1.into();
        let cond_2 = cond_2.into();
        Condition::and(
            Condition::or(cond_1.clone(), cond_2.clone()),
            Condition::not(Condition::and(cond_1, cond_2)),
        )
    }

    /// Creates an equivalente condition to a logical implication.
    /// `x ==> y` iff `not(x) or y`
    pub fn implication(cond_1: impl Into<Condition<P>>, cond_2: impl Into<Condition<P>>) -> Self {
        Condition::or(Condition::not(cond_1), cond_2)
    }

    /// Creates an equivalente condition to a logical equivalence.
    /// `x <==> y` iff `(x ==> y) or (y ==> x)`
    pub fn equivalence(cond_1: impl Into<Condition<P>>, cond_2: impl Into<Condition<P>>) -> Self {
        let cond_1 = cond_1.into();
        let cond_2 = cond_2.into();
        Condition::and(
            Condition::implication(cond_1.clone(), cond_2.clone()),
            Condition::implication(cond_2, cond_1),
        )
    }
}
impl<P> Condition<P> {
    pub fn is_condition(&self) -> bool {
        matches!(self, Self::Comparison(_))
    }
}

/// Represents a comparison that can be used in an Sql where clause.
/// Note that an [`Comparison`] is a [`Condition`] and therefore can be turned into one.
#[derive(Debug, Clone, PartialEq)]
pub enum Comparison<P = ParsedArgType> {
    /// `a > b`
    Greater(MathExpression<P>, MathExpression<P>),
    /// `a >= b` / `a > b or |a - b| < epsilon`
    GreaterEqual(MathExpression<P>, MathExpression<P>, Option<f64>),
    /// `a < b`
    Less(MathExpression<P>, MathExpression<P>),
    /// `a <= b` / `a < b or |a - b| < epsilon`
    LessEqual(MathExpression<P>, MathExpression<P>, Option<f64>),
    /// `a = b` / `|a - b| < epsilon`
    Equal(MathExpression<P>, MathExpression<P>, Option<f64>),
    /// `a != b` / `|a - b| >= epsilon`
    NotEqual(MathExpression<P>, MathExpression<P>, Option<f64>),
}

impl<P: Display> Display for Comparison<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (l, r) = self.get_left_right_expr();
        write!(
            f,
            "{l} {} {r}",
            match self {
                Comparison::Greater(_, _) => ">",
                Comparison::GreaterEqual(_, _, _) => ">=",
                Comparison::Less(_, _) => "<",
                Comparison::LessEqual(_, _, _) => ">=",
                Comparison::Equal(_, _, _) => "=",
                Comparison::NotEqual(_, _, _) => "!=",
            }
        )
    }
}

impl<P> Comparison<P> {
    /// Gets the left and right [`MathExpression`]s of the given comparison
    pub fn get_left_right_expr(&self) -> (&MathExpression<P>, &MathExpression<P>) {
        match self {
            Comparison::Greater(left_expr, right_expr)
            | Comparison::GreaterEqual(left_expr, right_expr, _)
            | Comparison::Less(left_expr, right_expr)
            | Comparison::LessEqual(left_expr, right_expr, _)
            | Comparison::Equal(left_expr, right_expr, _)
            | Comparison::NotEqual(left_expr, right_expr, _) => (left_expr, right_expr),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MathExpression<P = ParsedArgType> {
    Primitif(P),
    // Unary op
    Negation(Box<MathExpression<P>>),
    Floor(Box<MathExpression<P>>),
    Ceil(Box<MathExpression<P>>),
    Abs(Box<MathExpression<P>>),
    Sqrt(Box<MathExpression<P>>),
    // Bin operations
    BinOperation {
        left: Box<MathExpression<P>>,
        op: ArithmOp,
        right: Box<MathExpression<P>>,
    },
}

impl<P: Display> Display for MathExpression<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parenthesis_fn = |val: &MathExpression<P>| -> String {
            if val.is_primitif() {
                val.to_string()
            } else {
                format!("({val})")
            }
        };

        write!(
            f,
            "{}",
            match self {
                MathExpression::Primitif(p) => p.to_string(),
                MathExpression::Negation(math_expression) =>
                    format!("-{}", parenthesis_fn(math_expression)),
                MathExpression::Floor(math_expression) =>
                    format!("floor({})", parenthesis_fn(math_expression)),
                MathExpression::Ceil(math_expression) =>
                    format!("ceil({})", parenthesis_fn(math_expression)),
                MathExpression::Abs(math_expression) => format!("|{math_expression}|"),
                MathExpression::Sqrt(math_expression) =>
                    format!("sqrt({})", parenthesis_fn(math_expression)),
                MathExpression::BinOperation { left, op, right } =>
                    format!("{} {op} {}", parenthesis_fn(left), parenthesis_fn(right)),
            }
        )
    }
}

impl<P> From<P> for MathExpression<P> {
    fn from(value: P) -> Self {
        MathExpression::Primitif(value)
    }
}

impl<P> MathExpression<P> {
    pub fn floor(expr: impl Into<MathExpression<P>>) -> Self {
        Self::Floor(Box::new(expr.into()))
    }
    pub fn ceil(expr: impl Into<MathExpression<P>>) -> Self {
        Self::Ceil(Box::new(expr.into()))
    }
    pub fn abs(expr: impl Into<MathExpression<P>>) -> Self {
        Self::Abs(Box::new(expr.into()))
    }
    pub fn negation(expr: impl Into<MathExpression<P>>) -> Self {
        Self::Negation(Box::new(expr.into()))
    }
    pub fn sqrt(expr: impl Into<MathExpression<P>>) -> Self {
        Self::Sqrt(Box::new(expr.into()))
    }

    pub fn primitif(arg: impl Into<P>) -> Self {
        Self::Primitif(arg.into())
    }

    pub fn is_primitif(&self) -> bool {
        matches!(self, Self::Primitif(_))
    }

    pub fn bin_operation(
        left: impl Into<MathExpression<P>>,
        op: ArithmOp,
        right: impl Into<MathExpression<P>>,
    ) -> Self {
        Self::BinOperation {
            left: Box::new(left.into()),
            op,
            right: Box::new(right.into()),
        }
    }

    pub fn get_operator_name(&self) -> String {
        match self {
            MathExpression::Primitif(_p) => "Primitif",
            MathExpression::Negation(_math_expression) => "Negation",
            MathExpression::Floor(_math_expression) => "Floor",
            MathExpression::Ceil(_math_expression) => "Ceil",
            MathExpression::Abs(_math_expression) => "Abs",
            MathExpression::Sqrt(_math_expression) => "Sqrt",
            MathExpression::BinOperation {
                left: _,
                op,
                right: _,
            } => match op {
                ArithmOp::Add => "Addition",
                ArithmOp::Subtract => "Substraction",
                ArithmOp::Power => "Exponentation",
                ArithmOp::Multiply => "Multiplication",
                ArithmOp::Divide => "Division",
                ArithmOp::Modulo => "Modulus",
            },
        }
        .to_string()
    }
}

impl<P: Clone> MathExpression<P> {
    pub fn get_all_primitives_rec(&self) -> Vec<P> {
        let mut prim_vec = Vec::new();

        self.map_primitives(&mut |prim| {
            prim_vec.push(prim.clone());
        });

        prim_vec
    }

    pub fn map_primitives(&self, map: &mut dyn FnMut(&P)) {
        match self {
            MathExpression::Primitif(prim) => map(prim),
            MathExpression::Negation(math_expression)
            | MathExpression::Floor(math_expression)
            | MathExpression::Ceil(math_expression)
            | MathExpression::Abs(math_expression)
            | MathExpression::Sqrt(math_expression) => {
                math_expression.map_primitives(map);
            }
            MathExpression::BinOperation { left, op: _, right } => {
                left.map_primitives(map);
                right.map_primitives(map);
            }
        };
    }

    pub fn get_prim_non_rec(&self) -> Option<P> {
        if let MathExpression::Primitif(p) = &self {
            Some(p.clone())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArithmOp {
    Add,
    Subtract,
    Power,
    Multiply,
    Divide,
    Modulo,
}

impl Display for ArithmOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ArithmOp::Add => "+",
                ArithmOp::Subtract => "-",
                ArithmOp::Power => "**",
                ArithmOp::Multiply => "*",
                ArithmOp::Divide => "/",
                ArithmOp::Modulo => "%",
            }
        )
    }
}

/// Used to correctly identify *parsed* arguments type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedArgType {
    PrimString(String),
    PrimNumeric(EqF64),
    Dataset,
    Function(ParsedFunction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFunction {
    pub name: String,
    pub first_arg: Box<MathExpression>,
    pub other_args: Vec<MathExpression>,
}

impl From<ParsedFunction> for ParsedArgType {
    fn from(value: ParsedFunction) -> Self {
        ParsedArgType::Function(value)
    }
}

impl ParsedArgType {
    pub fn prim_string(value: impl ToString) -> Self {
        ParsedArgType::PrimString(value.to_string())
    }

    pub fn prim_numeric(value: f64) -> Self {
        ParsedArgType::PrimNumeric(value.into())
    }

    pub fn function(
        name: impl ToString,
        first_arg: impl Into<MathExpression>,
        args: Vec<impl Into<MathExpression>>,
    ) -> Self {
        let args = args.into_iter().map(|m| m.into()).collect();
        ParsedFunction {
            name: name.to_string(),
            first_arg: Box::new(first_arg.into()),
            other_args: args,
        }
        .into()
    }

    pub fn invariant(name: impl ToString) -> Self {
        ParsedFunction {
            name: name.to_string(),
            first_arg: Box::new(ParsedArgType::Dataset.into()),
            other_args: Vec::new(),
        }
        .into()
    }
}

impl Display for ParsedArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ParsedArgType::PrimString(v) => v.to_string(),
                ParsedArgType::PrimNumeric(v) => v.to_string(),
                ParsedArgType::Dataset => "G".to_string(),
                ParsedArgType::Function(function) => {
                    format!(
                        "{}({}{})",
                        function.name,
                        function.first_arg,
                        if function.other_args.is_empty() {
                            "".to_string()
                        } else {
                            let mut args_iter = function.other_args.iter();
                            let mut args_str = args_iter.next().expect("not empty").to_string();

                            for other in args_iter {
                                args_str.push_str(&format!(",{other}"));
                            }

                            args_str
                        }
                    )
                }
            }
        )
    }
}

impl<P> From<Comparison<P>> for Condition<P> {
    fn from(val: Comparison<P>) -> Self {
        Condition::Comparison(val)
    }
}
