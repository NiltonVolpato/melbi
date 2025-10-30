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
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_constant_negative_int() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "-42").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -42);
}

#[test]
fn test_constant_float() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "3.14").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_float().unwrap(), 3.14);
}

#[test]
fn test_constant_bool_true() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "true").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_constant_bool_false() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "false").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_constant_string() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, r#""hello""#).unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
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
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn test_int_subtraction() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "10 - 4").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 6);
}

#[test]
fn test_int_multiplication() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "3 * 4").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 12);
}

#[test]
fn test_int_division() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "10 / 2").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn test_int_division_truncates() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "7 / 3").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn test_int_power() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "2 ^ 10").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 1024);
}

#[test]
fn test_int_power_zero() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "5 ^ 0").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 1);
}

#[test]
fn test_int_division_by_zero() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "10 / 0").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]);
    assert!(matches!(result, Err(EvalError::DivisionByZero { .. })));
}

#[test]
fn test_int_wrapping_overflow_add() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "9223372036854775807 + 1").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), i64::MIN);
}

#[test]
fn test_int_wrapping_overflow_mul() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "9223372036854775807 * 2").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
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
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert!((result.as_float().unwrap() - 5.14).abs() < 0.0001);
}

#[test]
fn test_float_subtraction() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "10.5 - 3.5").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_float().unwrap(), 7.0);
}

#[test]
fn test_float_multiplication() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "2.5 * 4.0").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_float().unwrap(), 10.0);
}

#[test]
fn test_float_division() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "10.0 / 3.0").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    let expected = 10.0 / 3.0;
    assert!((result.as_float().unwrap() - expected).abs() < 0.0001);
}

#[test]
fn test_float_division_by_zero() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "10.0 / 0.0").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert!(result.as_float().unwrap().is_infinite());
}

#[test]
fn test_float_power() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "2.0 ^ 3.0").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
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
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 20);
}

#[test]
fn test_deeply_nested() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "((1 + 2) * (3 + 4)) - (5 * 6)").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), (1 + 2) * (3 + 4) - (5 * 6));
}

#[test]
fn test_operator_precedence() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Verify that * binds tighter than +
    let parsed = parser::parse(&arena, "2 + 3 * 4").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 14); // Not 20

    // Verify that ^ binds tighter than *
    let parsed = parser::parse(&arena, "2 * 3 ^ 2").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
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
    let typed =
        analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).expect("Type-check failed");

    // With default limit of 1000, this should succeed (100 < 1000)
    let result = eval(type_manager, &arena, &typed, &[], &[]);
    assert!(result.is_ok());

    // But with a lower limit of 50, it should fail
    let result = eval_with_limits(type_manager, &arena, &typed, &[], &[], 50);
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
    let typed =
        analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).expect("Type-check failed");

    // With custom limit of 100, this should succeed
    let result = eval_with_limits(type_manager, &arena, &typed, &[], &[], 100);
    assert!(result.is_ok());

    // But with limit of 40, it should fail
    let result = eval_with_limits(type_manager, &arena, &typed, &[], &[], 40);
    assert!(matches!(result, Err(EvalError::StackOverflow { .. })));
}

// ============================================================================
// Variables (Runtime Parameters)
// ============================================================================

#[test]
fn test_variable_simple_lookup() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let var_types = [("x", type_manager.int())];
    let parsed = parser::parse(&arena, "x").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &var_types).unwrap();

    let var_values = [("x", Value::int(type_manager, 42))];
    let result = eval(type_manager, &arena, &typed, &[], &var_values).unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_variable_in_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let var_types = [("x", type_manager.int())];
    let parsed = parser::parse(&arena, "x * 2 + 10").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &var_types).unwrap();

    let var_values = [("x", Value::int(type_manager, 5))];
    let result = eval(type_manager, &arena, &typed, &[], &var_values).unwrap();
    assert_eq!(result.as_int().unwrap(), 20); // 5 * 2 + 10 = 20
}

#[test]
fn test_multiple_variables() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let var_types = [("x", type_manager.int()), ("y", type_manager.int())];
    let parsed = parser::parse(&arena, "x + y * 2").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &var_types).unwrap();

    let var_values = [
        ("x", Value::int(type_manager, 10)),
        ("y", Value::int(type_manager, 20)),
    ];
    let result = eval(type_manager, &arena, &typed, &[], &var_values).unwrap();
    assert_eq!(result.as_int().unwrap(), 50); // 10 + 20 * 2 = 50
}

#[test]
fn test_variable_float() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let var_types = [("price", type_manager.float())];
    let parsed = parser::parse(&arena, "price * 1.2").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &var_types).unwrap();

    let var_values = [("price", Value::float(type_manager, 100.0))];
    let result = eval(type_manager, &arena, &typed, &[], &var_values).unwrap();
    assert_eq!(result.as_float().unwrap(), 120.0);
}

#[test]
fn test_variable_bool() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let var_types = [("flag", type_manager.bool())];
    let parsed = parser::parse(&arena, "flag").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &var_types).unwrap();

    let var_values = [("flag", Value::bool(type_manager, true))];
    let result = eval(type_manager, &arena, &typed, &[], &var_values).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_variable_string() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let var_types = [("name", type_manager.str())];
    let parsed = parser::parse(&arena, "name").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &var_types).unwrap();

    let var_values = [("name", Value::str(&arena, type_manager.str(), "Alice"))];
    let result = eval(type_manager, &arena, &typed, &[], &var_values).unwrap();
    assert_eq!(result.as_str().unwrap(), "Alice");
}

// ============================================================================
// Globals (Constants and Built-in Functions)
// ============================================================================

#[test]
fn test_global_constant_pi() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let globals_types = [("PI", type_manager.float())];
    let parsed = parser::parse(&arena, "PI * 2.0").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &globals_types, &[]).unwrap();

    let globals_values = [("PI", Value::float(type_manager, 3.14159))];
    let result = eval(type_manager, &arena, &typed, &globals_values, &[]).unwrap();
    assert!((result.as_float().unwrap() - 6.28318).abs() < 0.0001);
}

#[test]
fn test_global_constant_with_variables() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let globals_types = [("PI", type_manager.float())];
    let var_types = [("radius", type_manager.float())];
    let parsed = parser::parse(&arena, "PI * radius * radius").unwrap();
    let typed =
        analyzer::analyze(type_manager, &arena, &parsed, &globals_types, &var_types).unwrap();

    let globals_values = [("PI", Value::float(type_manager, 3.14159))];
    let var_values = [("radius", Value::float(type_manager, 5.0))];
    let result = eval(type_manager, &arena, &typed, &globals_values, &var_values).unwrap();

    // Area = PI * r^2 = 3.14159 * 5 * 5 = 78.53975
    assert!((result.as_float().unwrap() - 78.53975).abs() < 0.001);
}

#[test]
fn test_multiple_globals() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let globals_types = [("PI", type_manager.float()), ("E", type_manager.float())];
    let parsed = parser::parse(&arena, "PI + E").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &globals_types, &[]).unwrap();

    let globals_values = [
        ("PI", Value::float(type_manager, 3.14159)),
        ("E", Value::float(type_manager, 2.71828)),
    ];
    let result = eval(type_manager, &arena, &typed, &globals_values, &[]).unwrap();
    assert!((result.as_float().unwrap() - 5.85987).abs() < 0.0001);
}

// ============================================================================
// Shadowing Tests (Variables vs Where Bindings)
// ============================================================================

#[test]
fn test_where_shadows_variable() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let var_types = [("x", type_manager.int())];
    let parsed = parser::parse(&arena, "x where { x = 5 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &var_types).unwrap();

    let var_values = [("x", Value::int(type_manager, 10))];
    let result = eval(type_manager, &arena, &typed, &[], &var_values).unwrap();

    // Inner x = 5 shadows outer x = 10
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn test_where_can_reference_variable() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let var_types = [("x", type_manager.int())];
    let parsed = parser::parse(&arena, "y where { y = x * 2 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &var_types).unwrap();

    let var_values = [("x", Value::int(type_manager, 10))];
    let result = eval(type_manager, &arena, &typed, &[], &var_values).unwrap();

    // y = x * 2 = 10 * 2 = 20
    assert_eq!(result.as_int().unwrap(), 20);
}
