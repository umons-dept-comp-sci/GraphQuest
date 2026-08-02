use std::{fmt::Display, hash::Hash};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValueTypeError {
    #[error("Could not parse \"{0}\" as a valid module type.")]
    UnknownType(String),
    #[error("Could not parse the value \"{1}\" into a {0} type.")]
    ParseImpossible(ValueType, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    Numeric,
    String,
    Bool,
    Graph,
}

impl ValueType {
    pub fn translate_into_const(
        &self,
        str: impl Into<String>,
    ) -> Result<ConstantValue, ValueTypeError> {
        let str = str.into();

        Ok(match self {
            ValueType::Numeric => ConstantValue::Numeric(match str.parse::<f64>() {
                Ok(num) => num,
                Err(_) => {
                    return Err(ValueTypeError::ParseImpossible(self.clone(), str));
                }
            }),
            ValueType::Bool => ConstantValue::Bool(match str.parse::<bool>() {
                Ok(val) => val,
                Err(_) => {
                    return Err(ValueTypeError::ParseImpossible(self.clone(), str));
                }
            }),
            ValueType::String | ValueType::Graph => ConstantValue::String(str),
        })
    }
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
    Identifier(String),
    Bool(bool),
}

impl Eq for ConstantValue {}

impl Hash for ConstantValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
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
