//! Runtime evaluation errors.
//!
//! These are errors that can occur during expression evaluation.
//! Note: Many error conditions (type mismatches, undefined variables in typed code, etc.)
//! are caught by the analyzer and will never occur if the expression is type-checked first.
//!
//! # Error Categories
//!
//! - **Runtime errors**: Validation/logic errors during evaluation that can be caught
//!   by the `otherwise` operator (e.g., division by zero, index out of bounds).
//!
//! - **Resource exceeded errors**: Fatal resource limit violations that cannot be caught
//!   (e.g., stack overflow). These propagate through `otherwise` to prevent hiding
//!   serious resource exhaustion issues.

use crate::String;
use crate::parser::Span;
use alloc::string::ToString;
use core::fmt;

/// Runtime evaluation error.
#[derive(Debug)]
pub enum EvalError {
    /// Runtime error that can be caught by the `otherwise` operator.
    Runtime(RuntimeError),

    /// Resource limit exceeded (cannot be caught by `otherwise`).
    ResourceExceeded(ResourceExceeded),
}

/// Runtime errors that can be caught by the `otherwise` operator.
///
/// These represent validation/logic errors during expression evaluation
/// that are part of normal program flow and can be recovered from using
/// the `otherwise` operator.
#[derive(Debug)]
pub enum RuntimeError {
    /// Division by zero (integer or float division).
    DivisionByZero { span: Option<Span> },

    /// Array or map index out of bounds.
    IndexOutOfBounds {
        index: i64,
        len: usize,
        span: Option<Span>,
    },

    /// Cast error (e.g., invalid UTF-8 when casting Bytes → Str).
    ///
    /// TODO(effects): When effect system is implemented, mark fallible casts
    /// with `!` effect and make them catchable with `otherwise`.
    CastError { message: String, span: Option<Span> },
}

/// Resource limit exceeded errors that cannot be caught.
///
/// These represent fatal resource exhaustion that terminates evaluation.
/// The `otherwise` operator does not catch these errors to prevent hiding
/// serious resource issues like stack overflow.
#[derive(Debug)]
pub enum ResourceExceeded {
    /// Evaluation recursion depth exceeded.
    StackOverflow { depth: usize, max_depth: usize },
    // Future resource limits:
    // MemoryExceeded { bytes: usize, max_bytes: usize },
    // TimeExceeded { millis: u64, max_millis: u64 },
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::Runtime(e) => write!(f, "{}", e),
            EvalError::ResourceExceeded(e) => write!(f, "{}", e),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::DivisionByZero { span } => {
                write!(f, "Division by zero")?;
                if let Some(span) = span {
                    write!(f, " at {}..{}", span.0.start, span.0.end)?;
                }
                Ok(())
            }
            RuntimeError::IndexOutOfBounds { index, len, span } => {
                write!(f, "Index {} out of bounds (length: {})", index, len)?;
                if let Some(span) = span {
                    write!(f, " at {}..{}", span.0.start, span.0.end)?;
                }
                Ok(())
            }
            RuntimeError::CastError { message, span } => {
                write!(f, "Cast error: {}", message)?;
                if let Some(span) = span {
                    write!(f, " at {}..{}", span.0.start, span.0.end)?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Display for ResourceExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceExceeded::StackOverflow { depth, max_depth } => {
                write!(
                    f,
                    "Evaluation stack overflow: depth {} exceeds maximum of {}",
                    depth, max_depth
                )
            }
        }
    }
}

// Convenient conversions for error construction
impl From<RuntimeError> for EvalError {
    fn from(e: RuntimeError) -> Self {
        EvalError::Runtime(e)
    }
}

impl From<ResourceExceeded> for EvalError {
    fn from(e: ResourceExceeded) -> Self {
        EvalError::ResourceExceeded(e)
    }
}

// Integration with CastError from casting module
impl From<crate::casting::CastError> for RuntimeError {
    fn from(e: crate::casting::CastError) -> Self {
        RuntimeError::CastError {
            message: e.to_string(),
            span: None,
        }
    }
}

impl From<crate::casting::CastError> for EvalError {
    fn from(e: crate::casting::CastError) -> Self {
        EvalError::Runtime(RuntimeError::from(e))
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EvalError {}

#[cfg(feature = "std")]
impl std::error::Error for RuntimeError {}

#[cfg(feature = "std")]
impl std::error::Error for ResourceExceeded {}
