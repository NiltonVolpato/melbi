//! Binary and unary operator implementations.

use crate::{
    evaluator::{ExecutionErrorKind, RuntimeError::*},
    parser::{BinaryOp, ComparisonOp, UnaryOp},
};

/// Evaluate a binary operation on two integers.
///
/// Uses wrapping arithmetic to prevent panics on overflow.
/// Division by zero returns an error.
pub(super) fn eval_binary_int(
    op: BinaryOp,
    left: i64,
    right: i64,
) -> Result<i64, ExecutionErrorKind> {
    match op {
        BinaryOp::Add => Ok(left.wrapping_add(right)),
        BinaryOp::Sub => Ok(left.wrapping_sub(right)),
        BinaryOp::Mul => Ok(left.wrapping_mul(right)),
        BinaryOp::Div => {
            if right == 0 {
                Err(DivisionByZero {}.into())
            } else {
                // Use wrapping_div to handle i64::MIN / -1 case
                Ok(left.wrapping_div(right))
            }
        }
        BinaryOp::Pow => {
            // Handle power specially to avoid overflow panics
            if right < 0 {
                // Negative exponents for integers result in 0 (floor division semantics)
                Ok(0)
            } else if right > u32::MAX as i64 {
                // Exponent too large, will overflow or underflow
                // Return 0 for simplicity (matches negative exponent behavior)
                Ok(0)
            } else {
                // Use wrapping_pow for safe exponentiation
                Ok(left.wrapping_pow(right as u32))
            }
        }
    }
}

/// Evaluate a binary operation on two floats.
///
/// Follows IEEE 754 semantics (produces inf/nan rather than panicking).
pub(super) fn eval_binary_float(op: BinaryOp, left: f64, right: f64) -> f64 {
    match op {
        BinaryOp::Add => left + right,
        BinaryOp::Sub => left - right,
        BinaryOp::Mul => left * right,
        BinaryOp::Div => left / right, // Division by zero produces inf
        BinaryOp::Pow => left.powf(right),
    }
}

/// Evaluate a unary operation on an integer.
///
/// Uses wrapping arithmetic for negation to prevent panics on overflow.
pub(super) fn eval_unary_int(op: UnaryOp, value: i64) -> i64 {
    match op {
        UnaryOp::Neg => value.wrapping_neg(),
        UnaryOp::Not => {
            // Type checker should have caught this
            debug_assert!(false, "Not operator on non-boolean type");
            unreachable!("Not operator on Int in type-checked expression")
        }
    }
}

/// Evaluate a unary operation on a float.
pub(super) fn eval_unary_float(op: UnaryOp, value: f64) -> f64 {
    match op {
        UnaryOp::Neg => -value,
        UnaryOp::Not => {
            // Type checker should have caught this
            debug_assert!(false, "Not operator on non-boolean type");
            unreachable!("Not operator on Float in type-checked expression")
        }
    }
}

/// Evaluate a unary operation on a boolean.
pub(super) fn eval_unary_bool(op: UnaryOp, value: bool) -> bool {
    match op {
        UnaryOp::Not => !value,
        UnaryOp::Neg => {
            // Type checker should have caught this
            debug_assert!(false, "Neg operator on non-numeric type");
            unreachable!("Neg operator on Bool in type-checked expression")
        }
    }
}

/// Evaluate a comparison operation on two integers.
pub(super) fn eval_comparison_int(op: ComparisonOp, left: i64, right: i64) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::Neq => left != right,
        ComparisonOp::Lt => left < right,
        ComparisonOp::Gt => left > right,
        ComparisonOp::Le => left <= right,
        ComparisonOp::Ge => left >= right,
    }
}

/// Evaluate a comparison operation on two floats.
pub(super) fn eval_comparison_float(op: ComparisonOp, left: f64, right: f64) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::Neq => left != right,
        ComparisonOp::Lt => left < right,
        ComparisonOp::Gt => left > right,
        ComparisonOp::Le => left <= right,
        ComparisonOp::Ge => left >= right,
    }
}

/// Evaluate a comparison operation on two booleans.
pub(super) fn eval_comparison_bool(op: ComparisonOp, left: bool, right: bool) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::Neq => left != right,
        ComparisonOp::Lt | ComparisonOp::Gt | ComparisonOp::Le | ComparisonOp::Ge => {
            // Type checker should have caught this
            debug_assert!(false, "Ordering comparison on Bool type");
            unreachable!("Ordering comparison on Bool in type-checked expression")
        }
    }
}

/// Evaluate a comparison operation on two strings.
pub(super) fn eval_comparison_string(op: ComparisonOp, left: &str, right: &str) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::Neq => left != right,
        ComparisonOp::Lt => left < right,
        ComparisonOp::Gt => left > right,
        ComparisonOp::Le => left <= right,
        ComparisonOp::Ge => left >= right,
    }
}

/// Evaluate a comparison operation on two byte slices.
pub(super) fn eval_comparison_bytes(op: ComparisonOp, left: &[u8], right: &[u8]) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::Neq => left != right,
        ComparisonOp::Lt => left < right,
        ComparisonOp::Gt => left > right,
        ComparisonOp::Le => left <= right,
        ComparisonOp::Ge => left >= right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluator::RuntimeError;

    #[test]
    fn test_int_add() {
        assert_eq!(eval_binary_int(BinaryOp::Add, 2, 3).unwrap(), 5);
        assert_eq!(eval_binary_int(BinaryOp::Add, -5, 3).unwrap(), -2);
    }

    #[test]
    fn test_int_sub() {
        assert_eq!(eval_binary_int(BinaryOp::Sub, 10, 4).unwrap(), 6);
        assert_eq!(eval_binary_int(BinaryOp::Sub, 3, 10).unwrap(), -7);
    }

    #[test]
    fn test_int_mul() {
        assert_eq!(eval_binary_int(BinaryOp::Mul, 3, 4).unwrap(), 12);
        assert_eq!(eval_binary_int(BinaryOp::Mul, -2, 5).unwrap(), -10);
    }

    #[test]
    fn test_int_div() {
        assert_eq!(eval_binary_int(BinaryOp::Div, 10, 2).unwrap(), 5);
        assert_eq!(eval_binary_int(BinaryOp::Div, 7, 3).unwrap(), 2);
    }

    #[test]
    fn test_int_div_by_zero() {
        let result = eval_binary_int(BinaryOp::Div, 10, 0);
        assert!(matches!(
            result.as_ref().map(|_| ()),
            Err(crate::evaluator::ExecutionErrorKind::Runtime(
                RuntimeError::DivisionByZero {}
            ))
        ));
    }

    #[test]
    fn test_int_pow() {
        assert_eq!(eval_binary_int(BinaryOp::Pow, 2, 10).unwrap(), 1024);
        assert_eq!(eval_binary_int(BinaryOp::Pow, 3, 3).unwrap(), 27);
        assert_eq!(eval_binary_int(BinaryOp::Pow, 5, 0).unwrap(), 1);
    }

    #[test]
    fn test_int_pow_negative_exponent() {
        // Negative exponents for integers return 0 (floor semantics)
        assert_eq!(eval_binary_int(BinaryOp::Pow, 2, -1).unwrap(), 0);
    }

    #[test]
    fn test_int_wrapping_overflow() {
        // Test that we wrap on overflow rather than panic
        let result = eval_binary_int(BinaryOp::Add, i64::MAX, 1).unwrap();
        assert_eq!(result, i64::MIN);

        let result = eval_binary_int(BinaryOp::Mul, i64::MAX, 2).unwrap();
        assert_eq!(result, -2);
    }

    #[test]
    fn test_float_add() {
        let result = eval_binary_float(BinaryOp::Add, 3.14, 2.0);
        assert!((result - 5.14).abs() < 0.0001);
    }

    #[test]
    fn test_float_div() {
        assert_eq!(eval_binary_float(BinaryOp::Div, 10.0, 3.0), 10.0 / 3.0);
    }

    #[test]
    fn test_float_div_by_zero() {
        // Float division by zero produces infinity (IEEE 754)
        let result = eval_binary_float(BinaryOp::Div, 10.0, 0.0);
        assert!(result.is_infinite() && result.is_sign_positive());
    }

    #[test]
    fn test_float_pow() {
        assert_eq!(eval_binary_float(BinaryOp::Pow, 2.0, 3.0), 8.0);
    }
}
