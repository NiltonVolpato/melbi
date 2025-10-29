//! Type casting validation and conversion
//!
//! This module defines the casting system for Melbi. Casting allows explicit
//! type conversions using the `as` operator.
//!
//! # Future Design Notes
//!
//! - Some casts are infallible (Int->Float)
//! - Some casts can fail (String->Int parsing) - will integrate with effect system
//! - Unit conversions may interact with casting (e.g., 10`MB` as `bytes`)
//!
//! # Current Status
//!
//! Currently, all casts are rejected. Implementation is TODO.

use crate::String;
use crate::types::Type;

/// Check if a cast from `source_type` to `target_type` is valid.
///
/// Returns `true` if the cast is allowed, `false` otherwise.
///
/// # Current Implementation
///
/// Currently returns `false` for all casts (no casts are implemented yet).
///
/// # Future Implementation
///
/// Will support casts like:
/// - Numeric conversions: Int -> Float, Float -> Int (lossy)
/// - String conversions: Int -> Str, Float -> Str, Bool -> Str
/// - Parsing: Str -> Int (fallible), Str -> Float (fallible)
/// - Bytes conversions: Str -> Bytes, Bytes -> Str
/// - Possibly: Array/Map element type conversions
///
/// TODO: Define comprehensive casting rules
/// TODO: Distinguish infallible vs fallible casts (for effect system)
pub fn is_cast_valid<'types>(
    _source_type: &'types Type<'types>,
    _target_type: &'types Type<'types>,
) -> bool {
    // TODO: Implement casting rules
    false
}

/// Perform type checking for a cast expression.
///
/// Returns `Ok(())` if cast is valid, `Err(reason)` otherwise.
pub fn validate_cast<'types>(
    source_type: &'types Type<'types>,
    target_type: &'types Type<'types>,
) -> Result<(), CastError> {
    if is_cast_valid(source_type, target_type) {
        Ok(())
    } else {
        Err(CastError::NotYetImplemented {
            from: crate::format!("{}", source_type),
            to: crate::format!("{}", target_type),
        })
    }
}

/// Errors that can occur during cast validation
#[derive(Debug, Clone)]
pub enum CastError {
    /// Casting is not yet implemented
    NotYetImplemented { from: String, to: String },
    // Future variants when casting is implemented:
    // InvalidCast { from, to, reason },
    // IncompatibleUnits { ... },
    // LossyCastRequiresExplicit { ... },
}

impl core::fmt::Display for CastError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CastError::NotYetImplemented { from, to } => {
                write!(f, "Casting from {} to {} is not yet implemented", from, to)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::manager::TypeManager;
    use bumpalo::Bump;

    #[test]
    fn test_all_casts_currently_unsupported() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        assert!(!is_cast_valid(tm.int(), tm.float()));
        assert!(!is_cast_valid(tm.float(), tm.int()));
        assert!(!is_cast_valid(tm.int(), tm.str()));
        assert!(!is_cast_valid(tm.str(), tm.int()));
        assert!(!is_cast_valid(tm.bool(), tm.int()));
    }

    #[test]
    fn test_validate_cast_returns_error() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        let result = validate_cast(tm.int(), tm.float());
        assert!(result.is_err());

        match result {
            Err(CastError::NotYetImplemented { from, to }) => {
                assert_eq!(from, "Int");
                assert_eq!(to, "Float");
            }
            _ => panic!("Expected NotYetImplemented error"),
        }
    }
}
