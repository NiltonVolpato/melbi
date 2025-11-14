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
use core::fmt;

/// Runtime evaluation error.
#[derive(Debug)]
pub enum ExecutionError {
    /// Runtime error that can be caught by the `otherwise` operator.
    Runtime(RuntimeError),

    /// Resource limit exceeded (cannot be caught by `otherwise`).
    ResourceExceeded(ResourceExceededError),
}

/// Runtime errors that can be caught by the `otherwise` operator.
///
/// These represent validation/logic errors during expression evaluation
/// that are part of normal program flow and can be recovered from using
/// the `otherwise` operator.
#[derive(Debug)]
pub enum RuntimeError {
    /// Division by zero (integer or float division).
    DivisionByZero { span: Span },

    /// Array or map index out of bounds.
    IndexOutOfBounds {
        index: i64,
        len: usize,
        span: Span,
    },

    /// Cast error (e.g., invalid UTF-8 when casting Bytes → Str).
    ///
    /// TODO(effects): When effect system is implemented, mark fallible casts
    /// with `!` effect and make them catchable with `otherwise`.
    CastError { message: String, span: Span },
}

/// Resource limit exceeded errors that cannot be caught.
///
/// These represent fatal resource exhaustion that terminates evaluation.
/// The `otherwise` operator does not catch these errors to prevent hiding
/// serious resource issues like stack overflow.
#[derive(Debug)]
pub enum ResourceExceededError {
    /// Evaluation recursion depth exceeded.
    StackOverflow {
        depth: usize,
        max_depth: usize,
        span: Span,
    },
    // Future resource limits:
    // MemoryExceeded { bytes: usize, max_bytes: usize, span: Span },
    // TimeExceeded { millis: u64, max_millis: u64, span: Span },
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionError::Runtime(e) => write!(f, "{}", e),
            ExecutionError::ResourceExceeded(e) => write!(f, "{}", e),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::DivisionByZero { span } => {
                write!(f, "Division by zero at {}", format_span(span))
            }
            RuntimeError::IndexOutOfBounds { index, len, span } => {
                write!(
                    f,
                    "Index {} out of bounds (length: {}) at {}",
                    index,
                    len,
                    format_span(span)
                )
            }
            RuntimeError::CastError { message, span } => {
                write!(f, "Cast error: {} at {}", message, format_span(span))
            }
        }
    }
}

impl fmt::Display for ResourceExceededError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceExceededError::StackOverflow {
                depth,
                max_depth,
                span,
            } => {
                write!(
                    f,
                    "Evaluation stack overflow: depth {} exceeds maximum of {} at {}",
                    depth,
                    max_depth,
                    format_span(span)
                )
            }
        }
    }
}

/// Format a span as a byte range for error messages.
///
/// Returns a string like "5..12" representing the byte range.
/// In the future, this could be enhanced to show line:column information
/// if source text is available.
fn format_span(span: &Span) -> String {
    alloc::format!("{}..{}", span.0.start, span.0.end)
}

// Convenient conversions for error construction
impl From<RuntimeError> for ExecutionError {
    fn from(e: RuntimeError) -> Self {
        ExecutionError::Runtime(e)
    }
}

impl From<ResourceExceededError> for ExecutionError {
    fn from(e: ResourceExceededError) -> Self {
        ExecutionError::ResourceExceeded(e)
    }
}

// Note: CastError from the casting module cannot be automatically converted
// to RuntimeError anymore because RuntimeError::CastError now requires a span.
// Callers must construct the error manually with the appropriate span.

#[cfg(feature = "std")]
impl std::error::Error for ExecutionError {}

#[cfg(feature = "std")]
impl std::error::Error for RuntimeError {}

#[cfg(feature = "std")]
impl std::error::Error for ResourceExceededError {}
