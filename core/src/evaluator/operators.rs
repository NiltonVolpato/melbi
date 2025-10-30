//! Binary and unary operator implementations.

use crate::{
    evaluator::EvalError,
    parser::{BinaryOp, Span},
};

/// Evaluate a binary operation on two integers.
///
/// Uses wrapping arithmetic to prevent panics on overflow.
/// Division by zero returns an error.
pub(super) fn eval_binary_int(
    op: BinaryOp,
    left: i64,
    right: i64,
    span: Option<Span>,
) -> Result<i64, EvalError> {
    match op {
        BinaryOp::Add => Ok(left.wrapping_add(right)),
        BinaryOp::Sub => Ok(left.wrapping_sub(right)),
        BinaryOp::Mul => Ok(left.wrapping_mul(right)),
        BinaryOp::Div => {
            if right == 0 {
                Err(EvalError::DivisionByZero { span })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_add() {
        assert_eq!(eval_binary_int(BinaryOp::Add, 2, 3, None).unwrap(), 5);
        assert_eq!(eval_binary_int(BinaryOp::Add, -5, 3, None).unwrap(), -2);
    }

    #[test]
    fn test_int_sub() {
        assert_eq!(eval_binary_int(BinaryOp::Sub, 10, 4, None).unwrap(), 6);
        assert_eq!(eval_binary_int(BinaryOp::Sub, 3, 10, None).unwrap(), -7);
    }

    #[test]
    fn test_int_mul() {
        assert_eq!(eval_binary_int(BinaryOp::Mul, 3, 4, None).unwrap(), 12);
        assert_eq!(eval_binary_int(BinaryOp::Mul, -2, 5, None).unwrap(), -10);
    }

    #[test]
    fn test_int_div() {
        assert_eq!(eval_binary_int(BinaryOp::Div, 10, 2, None).unwrap(), 5);
        assert_eq!(eval_binary_int(BinaryOp::Div, 7, 3, None).unwrap(), 2);
    }

    #[test]
    fn test_int_div_by_zero() {
        let result = eval_binary_int(BinaryOp::Div, 10, 0, None);
        assert!(matches!(result, Err(EvalError::DivisionByZero { .. })));
    }

    #[test]
    fn test_int_pow() {
        assert_eq!(eval_binary_int(BinaryOp::Pow, 2, 10, None).unwrap(), 1024);
        assert_eq!(eval_binary_int(BinaryOp::Pow, 3, 3, None).unwrap(), 27);
        assert_eq!(eval_binary_int(BinaryOp::Pow, 5, 0, None).unwrap(), 1);
    }

    #[test]
    fn test_int_pow_negative_exponent() {
        // Negative exponents for integers return 0 (floor semantics)
        assert_eq!(eval_binary_int(BinaryOp::Pow, 2, -1, None).unwrap(), 0);
    }

    #[test]
    fn test_int_wrapping_overflow() {
        // Test that we wrap on overflow rather than panic
        let result = eval_binary_int(BinaryOp::Add, i64::MAX, 1, None).unwrap();
        assert_eq!(result, i64::MIN);

        let result = eval_binary_int(BinaryOp::Mul, i64::MAX, 2, None).unwrap();
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
