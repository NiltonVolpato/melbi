mod parsed_expr;
pub mod parser;
mod syntax;

// Re-export the parser and rule enum for external use
pub use parser::ExpressionParser;
pub use parser::Rule;
pub use parser::parse;

pub use parsed_expr::{Expr, Literal, ParsedExpr, TypeExpr};
pub use syntax::{BinaryOp, Span, UnaryOp};

#[cfg(test)]
mod literals_test;

#[cfg(test)]
mod parse_test;

#[cfg(test)]
mod rule_valid_test;

#[cfg(test)]
mod precedence_test;
