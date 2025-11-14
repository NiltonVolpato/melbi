//! Public error types for the Melbi API.
//!
//! This module defines the stable error types exposed to library users.
//! Internal errors are converted to these public types at API boundaries.
//!
//! See docs/design/error-handling.md for the complete design.

use crate::parser::Span;
use crate::{String, Vec, format};

#[cfg(feature = "std")]
use std::fmt;

#[cfg(not(feature = "std"))]
use core::fmt;

/// Public error type for all Melbi operations.
///
/// This is the stable error type exposed to library users. Internal error
/// representations may change, but this public API remains stable.
#[derive(Debug)]
pub enum Error {
    /// Invalid API usage (e.g., null pointer, invalid UTF-8, wrong arena).
    Api(String),

    /// Compilation errors (parse errors, type errors).
    ///
    /// Contains one or more diagnostics with source locations and context.
    Compilation { diagnostics: Vec<Diagnostic> },

    /// Runtime errors during evaluation (e.g., division by zero, index out of bounds).
    Runtime(String),

    /// Resource limits exceeded (e.g., stack overflow, iteration limit).
    ResourceExceeded(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Api(msg) => write!(f, "API error: {}", msg),
            Error::Compilation { diagnostics } => {
                let error_count = diagnostics
                    .iter()
                    .filter(|d| d.severity == Severity::Error)
                    .count();
                write!(f, "Compilation failed with {} error(s)", error_count)
            }
            Error::Runtime(msg) => write!(f, "Runtime error: {}", msg),
            Error::ResourceExceeded(msg) => write!(f, "Resource limit exceeded: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

/// A diagnostic message (error, warning, or info) with source location.
///
/// Maps cleanly to LSP diagnostics for IDE integration.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level (error, warning, info).
    pub severity: Severity,

    /// Primary diagnostic message.
    pub message: String,

    /// Source location of the primary issue.
    pub span: Span,

    /// Related locations that provide additional context.
    pub related: Vec<RelatedInfo>,

    /// Optional help text suggesting how to fix the issue.
    pub help: Option<String>,

    /// Optional error code (e.g., "E001") for documentation lookup.
    pub code: Option<String>,
}

/// Severity level for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Error - compilation cannot succeed.
    Error,
    /// Warning - suspicious code that might be wrong.
    Warning,
    /// Info - informational message.
    Info,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}

/// Related information for a diagnostic (e.g., "defined here", "inferred here").
#[derive(Debug, Clone)]
pub struct RelatedInfo {
    /// Source location of the related information.
    pub span: Span,

    /// Message explaining the relevance.
    pub message: String,
}

// ============================================================================
// Conversion from internal errors
// ============================================================================

impl From<crate::errors::Error> for Error {
    fn from(err: crate::errors::Error) -> Self {
        // For now, convert to a simple compilation error
        // TODO: Implement proper conversion with spans and context
        Error::Compilation {
            diagnostics: crate::Vec::from([Diagnostic {
                severity: Severity::Error,
                message: format!("{:?}", err.kind),
                span: Span(0..0), // Default span - could extract from ErrorKind variants
                related: crate::Vec::new(),
                help: None,
                code: None,
            }]),
        }
    }
}

impl From<pest::error::Error<crate::parser::Rule>> for Error {
    fn from(err: pest::error::Error<crate::parser::Rule>) -> Self {
        Error::Compilation {
            diagnostics: crate::Vec::from([Diagnostic {
                severity: Severity::Error,
                message: format!("Parse error: {}", err),
                span: Span(0..0), // pest errors have their own position tracking
                related: crate::Vec::new(),
                help: None,
                code: None,
            }]),
        }
    }
}

impl From<crate::evaluator::ExecutionError> for Error {
    fn from(err: crate::evaluator::ExecutionError) -> Self {
        use crate::evaluator::ExecutionErrorKind::*;
        match err.kind {
            ResourceExceeded(res_err) => Error::ResourceExceeded(format!("{}", res_err)),
            Runtime(runtime_err) => Error::Runtime(format!("{}", runtime_err)),
        }
    }
}
