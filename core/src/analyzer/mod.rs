pub mod analyzer;
#[allow(dead_code)]
pub mod typed_expr;

#[cfg(test)]
mod analyzer_test;

pub use analyzer::analyze;
