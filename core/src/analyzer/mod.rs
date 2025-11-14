pub mod analyzer;
pub mod typed_expr;
pub mod error;

#[cfg(test)]
mod analyzer_test;

pub use analyzer::analyze;
pub use error::{TypeError, TypeErrorKind};
