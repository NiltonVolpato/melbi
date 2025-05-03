pub mod parser;

// Re-export the parser and rule enum for external use
pub use parser::ExpressionParser;
pub use parser::Rule;

#[cfg(test)]
mod parse_test;
