use std::{fmt::Display, hash::Hash};

use thiserror::Error;

use crate::EqF64;

#[derive(Error, Debug)]
pub enum ValueTypeError {
    #[error("Could not parse \"{0}\" as a valid type.")]
    UnknownType(String),
    #[error("Could not parse the value \"{1}\" into a {0} type.")]
    ParseImpossible(ValueType, String),
}

/// Enum containing all types that can be returned by functions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    /// Numerics are real numbers ([`f64`] in Rust).
    Numeric,
    /// Strings are string values ([`String`] in Rust).
    String,
    /// Bool are boolean values ([`bool`] in Rust).
    Bool,
    /// Graph are string values that are actually simple undirected graphs encoded using Nauty's g6 format ([`String`] in Rust).
    Graph,
}

impl ValueType {
    /// Using the current [`ValueType`], tries to parse the given string into an equivalent [`ConstantValue`].
    /// # Errors
    /// Returns an [`ValueTypeError::ParseImpossible`] if the given string cannot be parsed into the expected [`ConstantValue`].
    pub fn translate_into_const(
        &self,
        str: impl Into<String>,
    ) -> Result<ConstantValue, ValueTypeError> {
        let str = str.into();

        Ok(match self {
            ValueType::Numeric => ConstantValue::Numeric(match str.parse::<f64>() {
                Ok(num) => num.into(),
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

/// Enum containing all values that can be given to a function as a parameter or as a return value of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstantValue {
    /// A real value.
    Numeric(EqF64),
    /// A string value.
    String(String),
    /// Identifiers are string values referring to a column or value in an SQL query.
    /// As such, they should only be used when working with SQL queries.
    Identifier(String),
    /// A boolean value.
    Bool(bool),
}

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
