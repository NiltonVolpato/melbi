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

use core::fmt;

use crate::String;
use crate::format;
use crate::parser::Span;

/// Execution error.
#[derive(Debug)]
pub struct ExecutionError {
    pub kind: ExecutionErrorKind,
    pub source: String,
    pub span: Span,
}

/// Variants of execution error.
#[derive(Debug)]
pub enum ExecutionErrorKind {
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
    DivisionByZero {},

    /// Array index out of bounds.
    IndexOutOfBounds { index: i64, len: usize },

    /// Map key not found during indexing operation.
    KeyNotFound { key_display: String },

    /// Cast error (e.g., invalid UTF-8 when casting Bytes → Str).
    ///
    /// TODO(effects): When effect system is implemented, mark fallible casts
    /// with `!` effect and make them catchable with `otherwise`.
    CastError { message: String },
}

/// Resource limit exceeded errors that cannot be caught.
///
/// These represent fatal resource exhaustion that terminates evaluation.
/// The `otherwise` operator does not catch these errors to prevent hiding
/// serious resource issues like stack overflow.
#[derive(Debug)]
pub enum ResourceExceededError {
    /// Evaluation recursion depth exceeded.
    StackOverflow { depth: usize, max_depth: usize },
    // Future resource limits:
    // MemoryExceeded { bytes: usize, max_bytes: usize },
    // TimeExceeded { millis: u64, max_millis: u64 },
}

impl ExecutionError {
    /// Convert to a Diagnostic for API boundary
    pub fn to_diagnostic(&self) -> crate::api::Diagnostic {
        use crate::api::{Diagnostic, Severity};

        let (message, code, help) = match &self.kind {
            ExecutionErrorKind::Runtime(RuntimeError::DivisionByZero {}) => (
                String::from("Division by zero"),
                Some("R001"),
                Some("Check that divisor is not zero before division"),
            ),
            ExecutionErrorKind::Runtime(RuntimeError::IndexOutOfBounds { index, len }) => (
                format!("Index {} out of bounds (length: {})", index, len),
                Some("R002"),
                Some("Ensure index is within valid range [0, length)"),
            ),
            ExecutionErrorKind::Runtime(RuntimeError::KeyNotFound { key_display }) => (
                format!("Key not found: {}", key_display),
                Some("R003"),
                Some("Use 'otherwise' to provide a fallback value for missing keys"),
            ),
            ExecutionErrorKind::Runtime(RuntimeError::CastError { message }) => (
                format!("Cast error: {}", message),
                Some("R004"),
                Some("Verify the value can be safely converted to the target type"),
            ),
            ExecutionErrorKind::ResourceExceeded(ResourceExceededError::StackOverflow {
                depth,
                max_depth,
            }) => (
                format!(
                    "Stack overflow: depth {} exceeds maximum of {}",
                    depth, max_depth
                ),
                Some("R005"),
                Some("Reduce recursion depth or increase stack limit"),
            ),
        };

        Diagnostic {
            severity: Severity::Error,
            message,
            span: self.span.clone(),
            related: crate::Vec::new(),
            help: help.map(|s| String::from(s)),
            code: code.map(|s| String::from(s)),
        }
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} @ {}", self.kind, format_span(&self.span))
    }
}

impl fmt::Display for ExecutionErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionErrorKind::Runtime(e) => write!(f, "{}", e),
            ExecutionErrorKind::ResourceExceeded(e) => write!(f, "{}", e),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::DivisionByZero {} => {
                write!(f, "Division by zero")
            }
            RuntimeError::IndexOutOfBounds { index, len } => {
                write!(f, "Index {} out of bounds (length: {})", index, len)
            }
            RuntimeError::KeyNotFound { key_display } => {
                write!(f, "Key not found: {}", key_display)
            }
            RuntimeError::CastError { message } => {
                write!(f, "Cast error: {}", message)
            }
        }
    }
}

impl fmt::Display for ResourceExceededError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceExceededError::StackOverflow { depth, max_depth } => {
                write!(
                    f,
                    "Evaluation stack overflow: depth {} exceeds maximum of {}",
                    depth, max_depth
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
impl From<RuntimeError> for ExecutionErrorKind {
    fn from(e: RuntimeError) -> Self {
        Self::Runtime(e)
    }
}

impl From<ResourceExceededError> for ExecutionErrorKind {
    fn from(e: ResourceExceededError) -> Self {
        Self::ResourceExceeded(e)
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
