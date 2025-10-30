//! Type casting validation and conversion
//!
//! This module defines the casting system for Melbi. Casting allows explicit
//! type conversions using the `as` operator.
//!
//! # Supported Casts (MVP)
//!
//! ## Numeric Conversions
//! - **Int → Float**: Infallible widening conversion
//! - **Float → Int**: Truncates toward zero, wraps on overflow, NaN→0, Inf→MAX/MIN
//!
//! ## Bytes ↔ String (UTF-8)
//! - **Str → Bytes**: Infallible UTF-8 encoding
//! - **Bytes → Str**: Fallible UTF-8 decoding (can fail on invalid UTF-8)
//!
//! # NOT Supported (Use Alternatives)
//!
//! - **Numeric → Str**: Use format strings: `f"{x}"`
//! - **Str → Numeric**: Use parsing functions (future packages/FFI)
//! - **Bool → Int**: Use if expressions: `if x then 1 else 0`
//! - **Non-UTF-8 encodings**: Future packages/FFI
//!
//! # Future Work
//!
//! - Strict mode: Float→Int fails on non-exact conversions (see docs/TODO.md)
//! - Effect system integration: Mark Bytes→Str as fallible with `!` effect
//! - Unit conversions: e.g., `10`MB` as `bytes``

use crate::String;
use crate::types::Type;
use crate::values::dynamic::Value;

/// Check if a cast from `source_type` to `target_type` is valid.
///
/// Returns `true` if the cast is allowed, `false` otherwise.
///
/// # Supported Casts
///
/// - Int → Float (infallible)
/// - Float → Int (infallible, truncates)
/// - Str → Bytes (infallible, UTF-8 encoding)
/// - Bytes → Str (fallible, UTF-8 decoding)
///
/// # TODO(effects)
///
/// When effect system is implemented, mark Bytes→Str as fallible (`!` effect).
pub fn is_cast_valid<'types>(
    source_type: &'types Type<'types>,
    target_type: &'types Type<'types>,
) -> bool {
    use Type::*;

    match (source_type, target_type) {
        // Numeric conversions
        (Int, Float) => true,
        (Float, Int) => true,

        // Bytes ↔ String (UTF-8)
        (Str, Bytes) => true,
        (Bytes, Str) => true,

        // All other casts are invalid
        _ => false,
    }
}

/// Perform type checking for a cast expression.
///
/// Returns `Ok(())` if cast is valid, `Err(reason)` otherwise.
///
/// This is called by the analyzer during type checking.
pub fn validate_cast<'types>(
    source_type: &'types Type<'types>,
    target_type: &'types Type<'types>,
) -> Result<(), CastError> {
    if is_cast_valid(source_type, target_type) {
        Ok(())
    } else {
        Err(CastError::InvalidCast {
            from: crate::format!("{}", source_type),
            to: crate::format!("{}", target_type),
        })
    }
}

/// Perform runtime type casting.
///
/// This function performs the actual type conversion at runtime.
/// The cast is assumed to be valid (checked by `validate_cast` during analysis).
///
/// # Supported Conversions
///
/// - **Int → Float**: Converts integer to floating point (may lose precision for very large integers)
/// - **Float → Int**: Truncates toward zero, wraps on overflow, NaN→0, Inf→i64::MAX/MIN
/// - **Str → Bytes**: UTF-8 encoding (always succeeds)
/// - **Bytes → Str**: UTF-8 decoding (fails on invalid UTF-8)
///
/// # Errors
///
/// Returns `CastError::InvalidUtf8` if Bytes→Str fails due to invalid UTF-8.
///
/// # Panics
///
/// Panics if the cast is not supported (should never happen if analyzer validated it).
///
/// # TODO(strict-mode)
///
/// In strict mode, Float→Int should fail on:
/// - Non-exact conversions (e.g., 3.7 → error)
/// - NaN, Infinity (currently wraps to 0 or MAX/MIN)
/// See docs/TODO.md for details.
pub fn perform_cast<'types, 'arena>(
    arena: &'arena bumpalo::Bump,
    value: Value<'types, 'arena>,
    target_type: &'types Type<'types>,
    type_manager: &'types crate::types::manager::TypeManager<'types>,
) -> Result<Value<'types, 'arena>, CastError> {
    use Type::*;

    match (value.ty, target_type) {
        // Int → Float
        (Int, Float) => {
            let int_val = value.as_int().expect("Value type matches");
            Ok(Value::float(type_manager, int_val as f64))
        }

        // Float → Int
        (Float, Int) => {
            let float_val = value.as_float().expect("Value type matches");

            // Handle special float values
            let int_val = if float_val.is_nan() {
                // NaN → 0
                0
            } else if float_val.is_infinite() {
                // Inf → MAX/MIN depending on sign
                if float_val.is_sign_positive() {
                    i64::MAX
                } else {
                    i64::MIN
                }
            } else {
                // Truncate toward zero
                // Use wrapping cast to handle overflow
                float_val as i64
            };

            Ok(Value::int(type_manager, int_val))
        }

        // Str → Bytes (UTF-8 encoding)
        (Str, Bytes) => {
            let str_val = value.as_str().expect("Value type matches");
            Ok(Value::bytes(arena, target_type, str_val.as_bytes()))
        }

        // Bytes → Str (UTF-8 decoding)
        (Bytes, Str) => {
            let bytes_val = value.as_bytes().expect("Value type matches");

            // Attempt UTF-8 decoding
            match core::str::from_utf8(bytes_val) {
                Ok(str_val) => Ok(Value::str(arena, target_type, str_val)),
                Err(e) => Err(CastError::InvalidUtf8 {
                    error: crate::format!("{}", e),
                }),
            }
        }

        // Invalid cast (should never happen if analyzer validated)
        _ => {
            debug_assert!(
                false,
                "Invalid cast from {:?} to {:?} - analyzer should have caught this",
                value.ty, target_type
            );
            Err(CastError::InvalidCast {
                from: crate::format!("{}", value.ty),
                to: crate::format!("{}", target_type),
            })
        }
    }
}

/// Errors that can occur during cast validation or execution
#[derive(Debug, Clone)]
pub enum CastError {
    /// Invalid cast (not allowed by type system)
    InvalidCast { from: String, to: String },

    /// Invalid UTF-8 sequence when casting Bytes → Str
    InvalidUtf8 { error: String },
}

impl core::fmt::Display for CastError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CastError::InvalidCast { from, to } => {
                write!(f, "Cannot cast from {} to {}", from, to)
            }
            CastError::InvalidUtf8 { error } => {
                write!(f, "Invalid UTF-8 sequence: {}", error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::manager::TypeManager;
    use bumpalo::Bump;

    // ========================================================================
    // Validation Tests (Analyzer)
    // ========================================================================

    #[test]
    fn test_numeric_casts_are_valid() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        assert!(is_cast_valid(tm.int(), tm.float()));
        assert!(is_cast_valid(tm.float(), tm.int()));
        assert!(validate_cast(tm.int(), tm.float()).is_ok());
        assert!(validate_cast(tm.float(), tm.int()).is_ok());
    }

    #[test]
    fn test_bytes_str_casts_are_valid() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        assert!(is_cast_valid(tm.str(), tm.bytes()));
        assert!(is_cast_valid(tm.bytes(), tm.str()));
        assert!(validate_cast(tm.str(), tm.bytes()).is_ok());
        assert!(validate_cast(tm.bytes(), tm.str()).is_ok());
    }

    #[test]
    fn test_unsupported_casts_are_invalid() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        // Numeric to string (use format strings instead)
        assert!(!is_cast_valid(tm.int(), tm.str()));
        assert!(!is_cast_valid(tm.float(), tm.str()));

        // String to numeric (use parsing functions instead)
        assert!(!is_cast_valid(tm.str(), tm.int()));
        assert!(!is_cast_valid(tm.str(), tm.float()));

        // Bool to int (use if expressions instead)
        assert!(!is_cast_valid(tm.bool(), tm.int()));

        // Identity casts (pointless)
        assert!(!is_cast_valid(tm.int(), tm.int()));
        assert!(!is_cast_valid(tm.str(), tm.str()));
    }

    #[test]
    fn test_validate_cast_returns_error_for_invalid() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        let result = validate_cast(tm.int(), tm.str());
        assert!(result.is_err());

        match result {
            Err(CastError::InvalidCast { from, to }) => {
                assert_eq!(from, "Int");
                assert_eq!(to, "Str");
            }
            _ => panic!("Expected InvalidCast error"),
        }
    }

    // ========================================================================
    // Runtime Conversion Tests (Evaluator)
    // ========================================================================

    #[test]
    fn test_int_to_float_cast() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        let int_val = Value::int(tm, 42);
        let result = perform_cast(&bump, int_val, tm.float(), tm).unwrap();

        assert_eq!(result.as_float().unwrap(), 42.0);
    }

    #[test]
    fn test_float_to_int_cast_truncates() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        // Positive truncation
        let float_val = Value::float(tm, 3.7);
        let result = perform_cast(&bump, float_val, tm.int(), tm).unwrap();
        assert_eq!(result.as_int().unwrap(), 3);

        // Negative truncation
        let float_val = Value::float(tm, -3.7);
        let result = perform_cast(&bump, float_val, tm.int(), tm).unwrap();
        assert_eq!(result.as_int().unwrap(), -3);
    }

    #[test]
    fn test_float_to_int_cast_special_values() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        // NaN → 0
        let nan_val = Value::float(tm, f64::NAN);
        let result = perform_cast(&bump, nan_val, tm.int(), tm).unwrap();
        assert_eq!(result.as_int().unwrap(), 0);

        // +Infinity → MAX
        let inf_val = Value::float(tm, f64::INFINITY);
        let result = perform_cast(&bump, inf_val, tm.int(), tm).unwrap();
        assert_eq!(result.as_int().unwrap(), i64::MAX);

        // -Infinity → MIN
        let neg_inf_val = Value::float(tm, f64::NEG_INFINITY);
        let result = perform_cast(&bump, neg_inf_val, tm.int(), tm).unwrap();
        assert_eq!(result.as_int().unwrap(), i64::MIN);
    }

    #[test]
    fn test_str_to_bytes_cast() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        let str_val = Value::str(&bump, tm.str(), "hello");
        let result = perform_cast(&bump, str_val, tm.bytes(), tm).unwrap();

        assert_eq!(result.as_bytes().unwrap(), b"hello");
    }

    #[test]
    fn test_bytes_to_str_cast_valid_utf8() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        let bytes_val = Value::bytes(&bump, tm.bytes(), b"hello");
        let result = perform_cast(&bump, bytes_val, tm.str(), tm).unwrap();

        assert_eq!(result.as_str().unwrap(), "hello");
    }

    #[test]
    fn test_bytes_to_str_cast_invalid_utf8() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        // Invalid UTF-8 sequence
        let invalid_bytes = &[0xFF, 0xFE, 0xFD];
        let bytes_val = Value::bytes(&bump, tm.bytes(), invalid_bytes);
        let result = perform_cast(&bump, bytes_val, tm.str(), tm);

        assert!(result.is_err());
        match result {
            Err(CastError::InvalidUtf8 { .. }) => {
                // Expected
            }
            _ => panic!("Expected InvalidUtf8 error"),
        }
    }

    #[test]
    fn test_utf8_roundtrip() {
        let bump = Bump::new();
        let tm = TypeManager::new(&bump);

        let original = "Hello, 世界! 🦀";
        let str_val = Value::str(&bump, tm.str(), original);

        // Str → Bytes
        let bytes_val = perform_cast(&bump, str_val, tm.bytes(), tm).unwrap();

        // Bytes → Str
        let str_val2 = perform_cast(&bump, bytes_val, tm.str(), tm).unwrap();

        assert_eq!(str_val2.as_str().unwrap(), original);
    }
}
