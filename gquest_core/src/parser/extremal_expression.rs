use std::fmt::Display;

// use thiserror::Error;

use crate::parser::parsed_expression::{MathExpression, ParsedArgType};

// #[derive(Error, Debug)]
// pub enum ExtremalSelectionError {
//     #[error("Cannot group the following expression with itself \"{0}\"")]
//     CombineWithItself(MathExpression),
// }

/// Represents a selection of extremal graphs.
#[derive(Clone, Debug, PartialEq)]
pub struct ExtremalSelection<P = ParsedArgType> {
    class_type: ClassType,
    aggregate_expr: MathExpression<P>,
    grouping_expr: Vec<MathExpression<P>>,
}

impl ExtremalSelection {
    pub fn new(
        class_type: ClassType,
        aggregate_expr: impl Into<MathExpression>,
        grouping_expr: Vec<impl Into<MathExpression>>,
    ) -> Self {
        let aggregate_expr = aggregate_expr.into();
        let grouping_expr: Vec<MathExpression> = Vec::from_iter(grouping_expr)
            .into_iter()
            .map(|f| f.into())
            .collect();

        // if grouping_expr.contains(&aggregate_expr) {
        //     return Err(ExtremalSelectionError::CombineWithItself(aggregate_expr));
        // }

        Self {
            class_type,
            aggregate_expr,
            grouping_expr,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClassType {
    Min,
    Max,
}

impl Display for ExtremalSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}({}{})",
            match self.class_type {
                ClassType::Min => "min",
                ClassType::Max => "max",
            },
            self.aggregate_expr,
            {
                if self.grouping_expr.is_empty() {
                    String::new()
                } else {
                    let mut res = String::from("; ");
                    let mut expressions = self.grouping_expr.iter();

                    res.push_str(&expressions.next().expect("not empty").to_string());
                    for expr in expressions {
                        res.push_str(&format!(", {expr}").to_string());
                    }
                    res
                }
            }
        )
    }
}
