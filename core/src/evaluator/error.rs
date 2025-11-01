//! Runtime evaluation errors.
//!
//! These are errors that can occur during expression evaluation.
//! Note: Many error conditions (type mismatches, undefined variables in typed code, etc.)
//! are caught by the analyzer and will never occur if the expression is type-checked first.

use crate::String;
use crate::parser::Span;
use core::fmt;

/// Runtime evaluation error.
#[derive(Debug)]
pub enum EvalError {
    /// Division by zero (integer or float).
    DivisionByZero { span: Option<Span> },

    /// Array or map index out of bounds.
    IndexOutOfBounds {
        index: i64,
        len: usize,
        span: Option<Span>,
    },

    /// Evaluation recursion depth exceeded.
    StackOverflow { depth: usize, max_depth: usize },

    /// Cast error (e.g., invalid UTF-8 when casting Bytes → Str).
    ///
    /// TODO(effects): When effect system is implemented, mark fallible casts
    /// with `!` effect and make them catchable with `otherwise`.
    CastError { message: String, span: Option<Span> },

    /// Argument count mismatch when calling a compiled expression.
    ArgumentCountMismatch { expected: usize, got: usize },

    /// Argument type mismatch when calling a compiled expression.
    ArgumentTypeMismatch {
        param_name: String,
        expected: String, // Type display name
        got: String,      // Type display name
    },
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::DivisionByZero { span } => {
                write!(f, "Division by zero")?;
                if let Some(span) = span {
                    write!(f, " at {}..{}", span.0.start, span.0.end)?;
                }
                Ok(())
            }
            EvalError::IndexOutOfBounds { index, len, span } => {
                write!(f, "Index {} out of bounds (length: {})", index, len)?;
                if let Some(span) = span {
                    write!(f, " at {}..{}", span.0.start, span.0.end)?;
                }
                Ok(())
            }
            EvalError::StackOverflow { depth, max_depth } => {
                write!(
                    f,
                    "Evaluation stack overflow: depth {} exceeds maximum of {}",
                    depth, max_depth
                )
            }
            EvalError::CastError { message, span } => {
                write!(f, "Cast error: {}", message)?;
                if let Some(span) = span {
                    write!(f, " at {}..{}", span.0.start, span.0.end)?;
                }
                Ok(())
            }
            EvalError::ArgumentCountMismatch { expected, got } => {
                write!(
                    f,
                    "Argument count mismatch: expected {} arguments, got {}",
                    expected, got
                )
            }
            EvalError::ArgumentTypeMismatch {
                param_name,
                expected,
                got,
            } => {
                write!(
                    f,
                    "Argument type mismatch for parameter '{}': expected {}, got {}",
                    param_name, expected, got
                )
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EvalError {}
