use std::fmt::Display;

use crate::parser::{extremal_expression::ExtremalSelection, parsed_expression::Condition};

pub mod extremal_expression;
pub mod parsed_expression;
pub mod query_parser;

#[derive(Debug, PartialEq)]
pub enum ExtremalCondition {
    ExtremalSelection(ExtremalSelection),
    Condition(Condition),
}

impl From<Condition> for ExtremalCondition {
    fn from(value: Condition) -> Self {
        Self::Condition(value)
    }
}

impl From<ExtremalSelection> for ExtremalCondition {
    fn from(value: ExtremalSelection) -> Self {
        Self::ExtremalSelection(value)
    }
}

impl Display for ExtremalCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtremalCondition::ExtremalSelection(extremal_selection) => extremal_selection.fmt(f),
            ExtremalCondition::Condition(condition) => condition.fmt(f),
        }
    }
}
