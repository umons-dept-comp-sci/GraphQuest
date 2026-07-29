use std::fmt::Display;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValueTypeError {
    #[error("Could not parse \"{0}\" as a valid module type.")]
    UnknownType(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    Numeric,
    String,
    Bool,
    Graph,
}

impl Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ValueType::Numeric => "Numeric",
                ValueType::String => "String",
                ValueType::Bool => "Bool",
                ValueType::Graph => "Graph",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstantValue {
    Numeric(f64),
    String(String),
    Bool(bool),
}

impl TryFrom<&str> for ValueType {
    type Error = ValueTypeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}

impl TryFrom<String> for ValueType {
    type Error = ValueTypeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value = value.to_lowercase();
        if value.eq("numeric") {
            Ok(ValueType::Numeric)
        } else if value.eq("graph") {
            Ok(ValueType::Graph)
        } else if value.eq("string") {
            Ok(ValueType::String)
        } else if value.eq("bool") {
            Ok(ValueType::Bool)
        } else {
            Err(ValueTypeError::UnknownType(value))
        }
    }
}
