use alloc::string::ToString;

use crate::api::{Diagnostic, Severity};
use crate::diagnostics::context::Context;
use crate::parser::Span;
use crate::types::Type;
use crate::{String, Vec, format};

/// Type error with context
#[derive(Debug)]
pub struct TypeError {
    pub kind: TypeErrorKind,
    pub source: String,
    pub context: Vec<Context>,
}

impl core::fmt::Display for TypeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let diagnostic = self.to_diagnostic();
        write!(f, "{}: {}", diagnostic.severity, diagnostic.message)?;

        if let Some(ref code) = diagnostic.code {
            write!(f, " [{}]", code)?;
        }

        if let Some(ref help) = diagnostic.help {
            write!(f, "\nhelp: {}", help)?;
        }

        Ok(())
    }
}

/// Specific kinds of type errors
#[derive(Debug)]
pub enum TypeErrorKind {
    /// Type mismatch between expected and found types
    TypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },
    /// Unbound/undefined variable
    UnboundVariable { name: String, span: Span },
    /// Unhandled error type
    UnhandledError { span: Span },
    /// Occurs check failed (infinite type)
    OccursCheck {
        type_var: String,
        ty: String,
        span: Span,
    },
    /// Type class constraint violation
    ConstraintViolation {
        ty: String,
        type_class: String,
        span: Span,
    },
    /// Field count mismatch in records
    FieldCountMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },
    /// Field name mismatch in records
    FieldNameMismatch {
        expected: String,
        found: String,
        span: Span,
    },
    /// Function parameter count mismatch
    FunctionParamCountMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },
    /// Cannot index into a non-indexable type
    NotIndexable { ty: String, span: Span },
    /// Field does not exist on record
    UnknownField {
        field: String,
        available_fields: Vec<String>,
        span: Span,
    },
    /// Cannot infer record type for field access
    CannotInferRecordType { field: String, span: Span },
    /// Tried to access field on non-record type
    NotARecord {
        ty: String,
        field: String,
        span: Span,
    },
    /// Invalid type expression in cast
    InvalidTypeExpression { message: String, span: Span },
    /// Invalid cast between types
    InvalidCast {
        from: String,
        to: String,
        reason: String,
        span: Span,
    },
    /// Duplicate parameter name in lambda
    DuplicateParameter { name: String, span: Span },
    /// Duplicate binding name in where clause
    DuplicateBinding { name: String, span: Span },
    /// Type is not formattable in format string
    NotFormattable { ty: String, span: Span },
    /// Unsupported language feature
    UnsupportedFeature {
        feature: String,
        suggestion: String,
        span: Span,
    },
    /// Generic type error (catch-all for other errors)
    Other { message: String, span: Span },
}

impl TypeErrorKind {
    /// Get the span of the error
    pub fn span(&self) -> Span {
        match self {
            TypeErrorKind::TypeMismatch { span, .. } => span.clone(),
            TypeErrorKind::UnboundVariable { span, .. } => span.clone(),
            TypeErrorKind::UnhandledError { span } => span.clone(),
            TypeErrorKind::OccursCheck { span, .. } => span.clone(),
            TypeErrorKind::ConstraintViolation { span, .. } => span.clone(),
            TypeErrorKind::FieldCountMismatch { span, .. } => span.clone(),
            TypeErrorKind::FieldNameMismatch { span, .. } => span.clone(),
            TypeErrorKind::FunctionParamCountMismatch { span, .. } => span.clone(),
            TypeErrorKind::NotIndexable { span, .. } => span.clone(),
            TypeErrorKind::UnknownField { span, .. } => span.clone(),
            TypeErrorKind::CannotInferRecordType { span, .. } => span.clone(),
            TypeErrorKind::NotARecord { span, .. } => span.clone(),
            TypeErrorKind::InvalidTypeExpression { span, .. } => span.clone(),
            TypeErrorKind::InvalidCast { span, .. } => span.clone(),
            TypeErrorKind::DuplicateParameter { span, .. } => span.clone(),
            TypeErrorKind::DuplicateBinding { span, .. } => span.clone(),
            TypeErrorKind::NotFormattable { span, .. } => span.clone(),
            TypeErrorKind::UnsupportedFeature { span, .. } => span.clone(),
            TypeErrorKind::Other { span, .. } => span.clone(),
        }
    }
}

impl TypeError {
    /// Create a new TypeError with no context
    pub fn new(kind: TypeErrorKind, source: String) -> Self {
        Self {
            kind,
            source,
            context: Vec::new(),
        }
    }

    /// Convert to a Diagnostic for API boundary
    pub fn to_diagnostic(&self) -> Diagnostic {
        let (message, code, help) = match &self.kind {
            TypeErrorKind::TypeMismatch {
                expected, found, ..
            } => (
                format!("Type mismatch: expected {}, found {}", expected, found),
                Some("E001"),
                Some("Types must match in this context"),
            ),
            TypeErrorKind::UnboundVariable { name, .. } => (
                format!("Undefined variable '{}'", name),
                Some("E002"),
                Some("Make sure the variable is declared before use"),
            ),
            TypeErrorKind::UnhandledError { .. } => (
                "Unhandled error type".to_string(),
                Some("E003"),
                Some("Use 'otherwise' to handle potential errors"),
            ),
            TypeErrorKind::OccursCheck { type_var, ty, .. } => (
                format!("Cannot construct infinite type: {} = {}", type_var, ty),
                Some("E004"),
                Some("This usually indicates a recursive type definition"),
            ),
            TypeErrorKind::ConstraintViolation { ty, type_class, .. } => (
                format!("Type '{}' does not implement {}", ty, type_class),
                Some("E005"),
                None,
            ),
            TypeErrorKind::FieldCountMismatch {
                expected, found, ..
            } => (
                format!(
                    "Record field count mismatch: expected {}, found {}",
                    expected, found
                ),
                Some("E006"),
                None,
            ),
            TypeErrorKind::FieldNameMismatch {
                expected, found, ..
            } => (
                format!(
                    "Record field name mismatch: expected '{}', found '{}'",
                    expected, found
                ),
                Some("E007"),
                None,
            ),
            TypeErrorKind::FunctionParamCountMismatch {
                expected, found, ..
            } => (
                format!(
                    "Function parameter count mismatch: expected {}, found {}",
                    expected, found
                ),
                Some("E008"),
                Some("Check the number of arguments in the function call"),
            ),
            TypeErrorKind::NotIndexable { ty, .. } => (
                format!("Cannot index into non-indexable type '{}'", ty),
                Some("E009"),
                Some("Only arrays, maps, bytes, and strings can be indexed"),
            ),
            TypeErrorKind::UnknownField {
                field,
                available_fields,
                ..
            } => (
                format!(
                    "Record does not have field '{}'. Available fields: {}",
                    field,
                    available_fields.join(", ")
                ),
                Some("E010"),
                Some("Check the field name for typos"),
            ),
            TypeErrorKind::CannotInferRecordType { field, .. } => (
                format!(
                    "Cannot infer record type for field access '.{}'. Row polymorphism not yet supported",
                    field
                ),
                Some("E011"),
                Some("Try adding a type annotation or casting to a concrete record type"),
            ),
            TypeErrorKind::NotARecord { ty, field, .. } => (
                format!(
                    "Cannot access field '{}' on non-record type '{}'",
                    field, ty
                ),
                Some("E012"),
                Some("Only record types support field access"),
            ),
            TypeErrorKind::InvalidTypeExpression { message, .. } => (
                format!("Invalid type expression: {}", message),
                Some("E013"),
                None,
            ),
            TypeErrorKind::InvalidCast {
                from, to, reason, ..
            } => (
                format!("Cannot cast from '{}' to '{}': {}", from, to, reason),
                Some("E014"),
                Some("Only certain type conversions are allowed"),
            ),
            TypeErrorKind::DuplicateParameter { name, .. } => (
                format!("Duplicate parameter name '{}'", name),
                Some("E015"),
                Some("Each parameter must have a unique name"),
            ),
            TypeErrorKind::DuplicateBinding { name, .. } => (
                format!("Duplicate binding name '{}'", name),
                Some("E016"),
                Some("Each binding in a where clause must have a unique name"),
            ),
            TypeErrorKind::NotFormattable { ty, .. } => (
                format!("Cannot format type '{}' in format string", ty),
                Some("E017"),
                Some("Function types cannot be formatted"),
            ),
            TypeErrorKind::UnsupportedFeature {
                feature,
                suggestion,
                ..
            } => (
                format!("{}", feature),
                Some("E018"),
                Some(suggestion.as_str()),
            ),
            TypeErrorKind::Other { message, .. } => (message.clone(), Some("E999"), None),
        };

        Diagnostic {
            severity: Severity::Error,
            message,
            span: self.kind.span(),
            related: self
                .context
                .iter()
                .map(|ctx| ctx.to_related_info())
                .collect(),
            help: help.map(|s| s.to_string()),
            code: code.map(|s| s.to_string()),
        }
    }

    /// Create a TypeError from a unification error
    pub fn from_unification_error(
        err: crate::types::unification::Error,
        span: Span,
        source: String,
    ) -> Self {
        use crate::types::unification::Error;

        let kind = match err {
            Error::OccursCheckFailed { type_var, ty } => {
                TypeErrorKind::OccursCheck { type_var, ty, span }
            }
            Error::FieldCountMismatch { expected, found } => TypeErrorKind::FieldCountMismatch {
                expected,
                found,
                span,
            },
            Error::FieldNameMismatch { expected, found } => TypeErrorKind::FieldNameMismatch {
                expected,
                found,
                span,
            },
            Error::FunctionParamCountMismatch { expected, found } => {
                TypeErrorKind::FunctionParamCountMismatch {
                    expected,
                    found,
                    span,
                }
            }
            Error::TypeMismatch { left, right } => TypeErrorKind::TypeMismatch {
                expected: left,
                found: right,
                span,
            },
        };

        Self::new(kind, source)
    }

}

/// Helper function to format types for error messages
pub fn format_type<'a>(ty: &'a Type<'a>) -> String {
    format!("{}", ty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_error_kind_span() {
        let span = Span(10..20);
        let kind = TypeErrorKind::UnboundVariable {
            name: "x".to_string(),
            span: span.clone(),
        };
        assert_eq!(kind.span(), span);
    }

    #[test]
    fn test_type_error_to_diagnostic() {
        let error = TypeError::new(
            TypeErrorKind::UnboundVariable {
                name: "x".to_string(),
                span: Span(10..20),
            },
            "test source".to_string(),
        );

        let diagnostic = error.to_diagnostic();
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(diagnostic.message.contains("Undefined variable 'x'"));
        assert_eq!(diagnostic.code, Some("E002".to_string()));
    }

    #[test]
    fn test_type_mismatch_diagnostic() {
        let error = TypeError::new(
            TypeErrorKind::TypeMismatch {
                expected: "Int".to_string(),
                found: "String".to_string(),
                span: Span(5..10),
            },
            "test source".to_string(),
        );

        let diagnostic = error.to_diagnostic();
        assert!(diagnostic.message.contains("Type mismatch"));
        assert!(diagnostic.message.contains("expected Int"));
        assert!(diagnostic.message.contains("found String"));
        assert_eq!(diagnostic.code, Some("E001".to_string()));
    }
}
