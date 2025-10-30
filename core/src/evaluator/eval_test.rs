//! Unit tests for the evaluator.

use super::*;
use crate::{analyzer, parser, types::manager::TypeManager};
use bumpalo::Bump;

// ============================================================================
// Constants
// ============================================================================

#[test]
fn test_constant_int() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "42").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_constant_negative_int() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "-42").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_int().unwrap(), -42);
}

#[test]
fn test_constant_float() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "3.14").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_float().unwrap(), 3.14);
}

#[test]
fn test_constant_bool_true() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "true").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_constant_bool_false() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "false").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_constant_string() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, r#""hello""#).unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_str().unwrap(), "hello");
}

// ============================================================================
// Integer Arithmetic
// ============================================================================

#[test]
fn test_int_addition() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "2 + 3").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn test_int_subtraction() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "10 - 4").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_int().unwrap(), 6);
}

#[test]
fn test_int_multiplication() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "3 * 4").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_int().unwrap(), 12);
}

#[test]
fn test_int_division() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "10 / 2").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn test_int_division_truncates() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "7 / 3").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn test_int_power() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "2 ^ 10").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_int().unwrap(), 1024);
}

#[test]
fn test_int_power_zero() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "5 ^ 0").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_int().unwrap(), 1);
}

#[test]
fn test_int_division_by_zero() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "10 / 0").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed);
    assert!(matches!(result, Err(EvalError::DivisionByZero { .. })));
}

#[test]
fn test_int_wrapping_overflow_add() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "9223372036854775807 + 1").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_int().unwrap(), i64::MIN);
}

#[test]
fn test_int_wrapping_overflow_mul() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "9223372036854775807 * 2").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_int().unwrap(), -2);
}

// ============================================================================
// Float Arithmetic
// ============================================================================

#[test]
fn test_float_addition() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "3.14 + 2.0").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert!((result.as_float().unwrap() - 5.14).abs() < 0.0001);
}

#[test]
fn test_float_subtraction() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "10.5 - 3.5").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_float().unwrap(), 7.0);
}

#[test]
fn test_float_multiplication() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "2.5 * 4.0").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_float().unwrap(), 10.0);
}

#[test]
fn test_float_division() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "10.0 / 3.0").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    let expected = 10.0 / 3.0;
    assert!((result.as_float().unwrap() - expected).abs() < 0.0001);
}

#[test]
fn test_float_division_by_zero() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "10.0 / 0.0").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert!(result.as_float().unwrap().is_infinite());
}

#[test]
fn test_float_power() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "2.0 ^ 3.0").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_float().unwrap(), 8.0);
}

// ============================================================================
// Nested Expressions
// ============================================================================

#[test]
fn test_nested_arithmetic() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "(2 + 3) * 4").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_int().unwrap(), 20);
}

#[test]
fn test_deeply_nested() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "((1 + 2) * (3 + 4)) - (5 * 6)").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_int().unwrap(), (1 + 2) * (3 + 4) - (5 * 6));
}

#[test]
fn test_operator_precedence() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Verify that * binds tighter than +
    let parsed = parser::parse(&arena, "2 + 3 * 4").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_int().unwrap(), 14); // Not 20

    // Verify that ^ binds tighter than *
    let parsed = parser::parse(&arena, "2 * 3 ^ 2").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed).unwrap();
    let result = eval(type_manager, &arena, &typed).unwrap();
    assert_eq!(result.as_int().unwrap(), 18); // Not 36
}

// ============================================================================
// Stack Depth Limit
// ============================================================================

#[test]
fn test_stack_depth_limit() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Create a deeply nested expression using actual operations: 1 + (1 + (1 + ...))
    // This creates real recursion depth, unlike just parentheses
    let mut source = String::from("1");
    for _ in 0..100 {
        source = format!("1 + ({})", source);
    }

    let parsed = parser::parse(&arena, &source).expect("Parse failed");
    let typed = analyzer::analyze(type_manager, &arena, &parsed).expect("Type-check failed");

    // With default limit of 1000, this should succeed (100 < 1000)
    let result = eval(type_manager, &arena, &typed);
    assert!(result.is_ok());

    // But with a lower limit of 50, it should fail
    let result = eval_with_limits(type_manager, &arena, &typed, 50);
    assert!(matches!(result, Err(EvalError::StackOverflow { .. })));
}

#[test]
fn test_custom_stack_depth_limit() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Create expression within custom limit
    let mut source = String::from("1");
    for _ in 0..50 {
        source = format!("({} + 1)", source);
    }

    let parsed = parser::parse(&arena, &source).expect("Parse failed");
    let typed = analyzer::analyze(type_manager, &arena, &parsed).expect("Type-check failed");

    // With custom limit of 100, this should succeed
    let result = eval_with_limits(type_manager, &arena, &typed, 100);
    assert!(result.is_ok());

    // But with limit of 40, it should fail
    let result = eval_with_limits(type_manager, &arena, &typed, 40);
    assert!(matches!(result, Err(EvalError::StackOverflow { .. })));
}
