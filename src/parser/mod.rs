pub mod parser;

// Re-export the parser and rule enum for external use
pub use parser::ExpressionParser;
pub use parser::Rule;
pub use parser::parse;

#[cfg(test)]
mod parse_test;

#[cfg(test)]
mod rule_valid_test;

#[cfg(test)]
mod precedence_test;
