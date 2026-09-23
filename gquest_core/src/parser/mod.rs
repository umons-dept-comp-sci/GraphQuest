use crate::parser::{extremal_expression::ExtremalSelection, parsed_expression::Condition};

pub mod extremal_expression;
pub mod parsed_expression;
pub mod query_parser;

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
