use std::fmt::Display;

pub mod utils;

pub mod data_handler;

pub mod parser;

pub mod database_handler;

pub mod engine;

#[derive(Clone, Copy, Debug)]
pub struct EqF64(pub f64);

impl PartialEq for EqF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for EqF64 {}

impl From<f64> for EqF64 {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl Display for EqF64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
