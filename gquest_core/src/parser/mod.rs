use std::fmt::Display;

use crate::parser::{
    extremal_expression::ExtremalSelection,
    parsed_expression::{Condition, ParsedArgType},
};

pub mod extremal_expression;
pub mod parsed_expression;
pub mod query_parser;

#[derive(Debug, PartialEq)]
pub enum ExtremalCondition<P = ParsedArgType> {
    ExtremalSelection(ExtremalSelection<P>),
    Condition(Condition<P>),
}

impl<P> From<Condition<P>> for ExtremalCondition<P> {
    fn from(value: Condition<P>) -> Self {
        Self::Condition(value)
    }
}

impl<P> From<ExtremalSelection<P>> for ExtremalCondition<P> {
    fn from(value: ExtremalSelection<P>) -> Self {
        Self::ExtremalSelection(value)
    }
}

impl Display for ExtremalCondition<ParsedArgType> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtremalCondition::ExtremalSelection(extremal_selection) => extremal_selection.fmt(f),
            ExtremalCondition::Condition(condition) => condition.fmt(f),
        }
    }
}
