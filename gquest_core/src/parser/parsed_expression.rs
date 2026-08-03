use std::fmt::Display;

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
}

/// Represents a **parsed** condition. Not yet typed checked.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition<P = ParsedArgType> {
    /// A simple comparison
    Operation(Comparison<P>),
    /// `x` or `y`
    Or(Box<Condition<P>>, Box<Condition<P>>),
    /// `x` and `y`
    And(Box<Condition<P>>, Box<Condition<P>>),
    // /// `x_0` and `x_1` and ... and `x_{n-1}`.
    // AndVec(Box<Condition<P>>, Vec<Condition<P>>),
    // /// `x_0` or `x_1` or ... or `x_{n-1}`.
    // OrVec(Box<Condition<P>>, Vec<Condition<P>>),
    /// not `x`
    Not(Box<Condition<P>>),
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

    // /// Simplifies the creation of the [`Condition::AndVec`] enum.
    // ///
    // /// If `conds` is empty, then `cond_1` will simply be returned.
    // pub fn and_vec(cond_1: impl Into<Condition<P>>, conds: Vec<impl Into<Condition<P>>>) -> Self {
    //     if conds.is_empty() {
    //         cond_1.into()
    //     } else {
    //         Self::AndVec(
    //             Box::new(cond_1.into()),
    //             conds.into_iter().map(|c| c.into()).collect(),
    //         )
    //     }
    // }

    // /// Simplifies the creation of the [`Condition::OrVec`] enum.
    // ///
    // /// If `conds` is empty, then `cond_1` will simply be returned.
    // pub fn or_vec(cond_1: impl Into<Condition>, conds: Vec<impl Into<Condition>>) -> Self {
    //     if conds.is_empty() {
    //         cond_1.into()
    //     } else {
    //         Self::OrVec(
    //             Box::new(cond_1.into()),
    //             conds.into_iter().map(|c| c.into()).collect(),
    //         )
    //     }
    // }
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

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedArgType {
    PrimString(String),
    PrimNumeric(f64),
    Dataset,
    Function(ParsedFunction),
}

#[derive(Debug, Clone, PartialEq)]
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
        ParsedArgType::PrimNumeric(value)
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
                ParsedArgType::Function(function) => format!(
                    "{}({:?},{:?})",
                    function.name, function.first_arg, function.other_args
                ),
            }
        )
    }
}

impl<P> From<Comparison<P>> for Condition<P> {
    fn from(val: Comparison<P>) -> Self {
        Condition::Operation(val)
    }
}
