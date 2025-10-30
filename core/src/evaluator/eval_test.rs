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
// Boolean Operators (Milestone 1.3) - Short-Circuit Evaluation
// ============================================================================

#[test]
fn test_boolean_and_true_true() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "true and true").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_boolean_and_true_false() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "true and false").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_boolean_and_false_true() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "false and true").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    // Right side not evaluated due to short-circuit
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_boolean_and_false_false() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "false and false").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    // Right side not evaluated due to short-circuit
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_boolean_or_true_true() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "true or true").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    // Right side not evaluated due to short-circuit
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_boolean_or_true_false() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "true or false").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    // Right side not evaluated due to short-circuit
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_boolean_or_false_true() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "false or true").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_boolean_or_false_false() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "false or false").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_boolean_chain_and() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "true and true and false").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_boolean_chain_or() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "false or false or true").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_boolean_mixed_chain() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    // 'and' has higher precedence than 'or'
    // So: true and false or true = (true and false) or true = false or true = true
    let parsed = parser::parse(&arena, "true and false or true").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_boolean_with_variables() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let var_types = [("x", type_manager.bool()), ("y", type_manager.bool())];
    let parsed = parser::parse(&arena, "x and y").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &var_types).unwrap();

    let var_values = [
        ("x", Value::bool(type_manager, true)),
        ("y", Value::bool(type_manager, false)),
    ];
    let result = eval(type_manager, &arena, &typed, &[], &var_values).unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_boolean_short_circuit_and_with_where() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    // false and (x where { x = true })
    // The where expression should not be evaluated due to short-circuit
    let parsed = parser::parse(&arena, "false and (x where { x = true })").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_boolean_short_circuit_or_with_where() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    // true or (x where { x = false })
    // The where expression should not be evaluated due to short-circuit
    let parsed = parser::parse(&arena, "true or (x where { x = false })").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
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

    let globals_types = [("E", type_manager.float()), ("PI", type_manager.float())];
    let parsed = parser::parse(&arena, "PI + E").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &globals_types, &[]).unwrap();

    let globals_values = [
        ("E", Value::float(type_manager, 2.71828)),
        ("PI", Value::float(type_manager, 3.14159)),
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

// ============================================================================
// Where Expressions (Local Scoping)
// ============================================================================

#[test]
fn test_where_simple() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "x where { x = 42 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_where_multiple_bindings() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "x + y where { x = 10, y = 20 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 30);
}

#[test]
fn test_where_sequential_binding() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // b can reference a (sequential binding)
    let parsed = parser::parse(&arena, "b where { a = 1, b = a + 1 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn test_where_sequential_binding_chain() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // c can reference b which references a
    let parsed = parser::parse(&arena, "c where { a = 1, b = a * 2, c = b + 1 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 3); // a=1, b=2, c=3
}

#[test]
fn test_where_complex_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "a + b + c where { a = 1, b = a * 2, c = b + 1 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 6); // 1 + 2 + 3 = 6
}

#[test]
fn test_where_nested_scopes() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "x + y where { x = 10 } where { y = 20 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 30);
}

#[test]
fn test_where_with_arithmetic() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "(a + b) * c where { a = 2, b = 3, c = 4 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 20); // (2 + 3) * 4 = 20
}

#[test]
fn test_where_with_float() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "x * y where { x = 2.5, y = 4.0 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_float().unwrap(), 10.0);
}

// ============================================================================
// Records (Milestone 2.2)
// ============================================================================

#[test]
fn test_record_empty() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "Record{}").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    let record = result.as_record().unwrap();
    assert_eq!(record.len(), 0);
    assert_eq!(format!("{}", result), "{}");
}

#[test]
fn test_record_simple() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "{ x = 42, y = 3.14 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    let record = result.as_record().unwrap();
    assert_eq!(record.len(), 2);

    let x = record.get("x").unwrap();
    assert_eq!(x.as_int().unwrap(), 42);

    let y = record.get("y").unwrap();
    assert!((y.as_float().unwrap() - 3.14).abs() < 0.0001);
}

#[test]
fn test_field_access_simple() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "{ x = 42 }.x").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_field_access_multiple_fields() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "{ a = 10, b = 20, c = 30 }.b").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 20);
}

#[test]
fn test_field_access_in_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "{ x = 5, y = 10 }.x + { x = 5, y = 10 }.y").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 15);
}

#[test]
fn test_record_with_where() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "{ x = a, y = b } where { a = 1, b = 2 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    let record = result.as_record().unwrap();

    let x = record.get("x").unwrap();
    assert_eq!(x.as_int().unwrap(), 1);

    let y = record.get("y").unwrap();
    assert_eq!(y.as_int().unwrap(), 2);
}

#[test]
fn test_nested_record() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed =
        parser::parse(&arena, "{ point = { x = 10, y = 20 }, name = \"origin\" }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    let outer = result.as_record().unwrap();

    let name = outer.get("name").unwrap();
    assert_eq!(name.as_str().unwrap(), "origin");

    let point = outer.get("point").unwrap();
    let point_rec = point.as_record().unwrap();

    let x = point_rec.get("x").unwrap();
    assert_eq!(x.as_int().unwrap(), 10);

    let y = point_rec.get("y").unwrap();
    assert_eq!(y.as_int().unwrap(), 20);
}

#[test]
fn test_nested_field_access() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "{ point = { x = 10, y = 20 } }.point.x").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 10);
}

#[test]
fn test_math_package_record() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Create Math record type with PI and E fields
    let math_ty = type_manager.record(&[("E", type_manager.float()), ("PI", type_manager.float())]);

    let globals_types = [("Math", math_ty)];
    let parsed = parser::parse(&arena, "Math.PI * 2.0 + Math.E").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &globals_types, &[]).unwrap();

    // Create Math record value with PI and E
    let math_value = Value::record(
        &arena,
        math_ty,
        &[
            ("E", Value::float(type_manager, 2.71828)),
            ("PI", Value::float(type_manager, 3.14159)),
        ],
    )
    .unwrap();

    let globals_values = [("Math", math_value)];
    let result = eval(type_manager, &arena, &typed, &globals_values, &[]).unwrap();

    // Math.PI * 2.0 + Math.E = 3.14159 * 2.0 + 2.71828 = 6.28318 + 2.71828 = 9.00146
    assert!((result.as_float().unwrap() - 9.00146).abs() < 0.0001);
}

#[test]
fn test_math_package_circle_area() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Create Math record type
    let math_ty = type_manager.record(&[("PI", type_manager.float())]);

    let globals_types = [("Math", math_ty)];
    let var_types = [("radius", type_manager.float())];
    let parsed = parser::parse(&arena, "Math.PI * radius * radius").unwrap();
    let typed =
        analyzer::analyze(type_manager, &arena, &parsed, &globals_types, &var_types).unwrap();

    // Create Math record value
    let math_value = Value::record(
        &arena,
        math_ty,
        &[("PI", Value::float(type_manager, 3.14159))],
    )
    .unwrap();

    let globals_values = [("Math", math_value)];
    let var_values = [("radius", Value::float(type_manager, 5.0))];
    let result = eval(type_manager, &arena, &typed, &globals_values, &var_values).unwrap();

    // Area = Math.PI * r^2 = 3.14159 * 5 * 5 = 78.53975
    assert!((result.as_float().unwrap() - 78.53975).abs() < 0.001);
}

// ================================
// Unary Operator Tests
// ================================

#[test]
fn test_unary_negation_int() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "-42").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -42);
}

#[test]
fn test_unary_negation_int_positive() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "-(42)").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -42);
}

#[test]
fn test_unary_double_negation() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "-(-5)").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn test_unary_negation_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "-(1 + 2)").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -3);
}

#[test]
fn test_unary_negation_float() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "-(3.14)").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert!((result.as_float().unwrap() + 3.14).abs() < 0.0001);
}

#[test]
fn test_unary_negation_float_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "-(2.5 + 1.5)").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert!((result.as_float().unwrap() + 4.0).abs() < 0.0001);
}

#[test]
fn test_unary_not_true() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "not true").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_unary_not_false() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "not false").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_unary_not_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "not (true and false)").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_unary_with_where() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "-x where { x = 42 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -42);
}

#[test]
fn test_unary_negation_wrapping() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    // Use string interpolation to build the source with i64::MIN
    let source = format!("-({})", i64::MIN);
    let parsed = parser::parse(&arena, &source).unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    // -i64::MIN wraps to i64::MIN
    assert_eq!(result.as_int().unwrap(), i64::MIN);
}

// ================================
// If/Else Expression Tests
// ================================

#[test]
fn test_if_true_branch() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "if true then 1 else 2").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 1);
}

#[test]
fn test_if_false_branch() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "if false then 1 else 2").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn test_if_with_variable() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let var_types = [("flag", type_manager.bool())];
    let parsed = parser::parse(&arena, "if flag then 10 else 20").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &var_types.as_ref()).unwrap();

    // Test with flag = true
    let var_values = [("flag", Value::bool(type_manager, true))];
    let result = eval(type_manager, &arena, &typed, &[], &var_values).unwrap();
    assert_eq!(result.as_int().unwrap(), 10);

    // Test with flag = false
    let var_values = [("flag", Value::bool(type_manager, false))];
    let result = eval(type_manager, &arena, &typed, &[], &var_values).unwrap();
    assert_eq!(result.as_int().unwrap(), 20);
}

#[test]
fn test_if_with_expression_condition() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "if true and false then 1 else 2").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn test_if_with_where() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "if x then 1 else 2 where { x = true }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 1);
}

#[test]
fn test_if_nested() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "if true then (if false then 1 else 2) else 3").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn test_if_float_branches() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "if true then 3.14 else 2.71").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert!((result.as_float().unwrap() - 3.14).abs() < 0.0001);
}

#[test]
fn test_if_string_branches() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, r#"if false then "yes" else "no""#).unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_str().unwrap(), "no");
}

#[test]
fn test_if_bool_branches() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "if true then true else false").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_if_with_complex_expressions() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "if true then (1 + 2) * 3 else 4 ^ 2").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 9);
}

// ================================
// Array Tests
// ================================

#[test]
fn test_array_empty() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "[]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 0);
}

#[test]
fn test_array_simple_int() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "[1, 2, 3]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_int().unwrap(), 1);
    assert_eq!(array.get(1).unwrap().as_int().unwrap(), 2);
    assert_eq!(array.get(2).unwrap().as_int().unwrap(), 3);
}

#[test]
fn test_array_simple_float() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "[3.14, 2.71, 1.41]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert!((array.get(0).unwrap().as_float().unwrap() - 3.14).abs() < 0.001);
    assert!((array.get(1).unwrap().as_float().unwrap() - 2.71).abs() < 0.001);
    assert!((array.get(2).unwrap().as_float().unwrap() - 1.41).abs() < 0.001);
}

#[test]
fn test_array_simple_bool() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "[true, false, true]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_bool().unwrap(), true);
    assert_eq!(array.get(1).unwrap().as_bool().unwrap(), false);
    assert_eq!(array.get(2).unwrap().as_bool().unwrap(), true);
}

#[test]
fn test_array_simple_string() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, r#"["a", "b", "c"]"#).unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_str().unwrap(), "a");
    assert_eq!(array.get(1).unwrap().as_str().unwrap(), "b");
    assert_eq!(array.get(2).unwrap().as_str().unwrap(), "c");
}

#[test]
fn test_array_with_expressions() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "[1 + 1, 2 * 2, 3 ^ 2]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_int().unwrap(), 2);
    assert_eq!(array.get(1).unwrap().as_int().unwrap(), 4);
    assert_eq!(array.get(2).unwrap().as_int().unwrap(), 9);
}

#[test]
fn test_array_nested() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "[[1, 2], [3, 4]]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);

    let inner1 = array.get(0).unwrap().as_array().unwrap();
    assert_eq!(inner1.len(), 2);
    assert_eq!(inner1.get(0).unwrap().as_int().unwrap(), 1);
    assert_eq!(inner1.get(1).unwrap().as_int().unwrap(), 2);

    let inner2 = array.get(1).unwrap().as_array().unwrap();
    assert_eq!(inner2.len(), 2);
    assert_eq!(inner2.get(0).unwrap().as_int().unwrap(), 3);
    assert_eq!(inner2.get(1).unwrap().as_int().unwrap(), 4);
}

#[test]
fn test_array_with_where() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "[x, y, z] where { x = 1, y = 2, z = 3 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_int().unwrap(), 1);
    assert_eq!(array.get(1).unwrap().as_int().unwrap(), 2);
    assert_eq!(array.get(2).unwrap().as_int().unwrap(), 3);
}

#[test]
fn test_array_with_variables() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let var_types = [
        ("x", type_manager.int()),
        ("y", type_manager.int()),
        ("z", type_manager.int()),
    ];
    let parsed = parser::parse(&arena, "[x, y, z]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &var_types).unwrap();

    let var_values = [
        ("x", Value::int(type_manager, 10)),
        ("y", Value::int(type_manager, 20)),
        ("z", Value::int(type_manager, 30)),
    ];
    let result = eval(type_manager, &arena, &typed, &[], &var_values).unwrap();
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_int().unwrap(), 10);
    assert_eq!(array.get(1).unwrap().as_int().unwrap(), 20);
    assert_eq!(array.get(2).unwrap().as_int().unwrap(), 30);
}

// ================================
// Array Indexing Tests
// ================================

#[test]
fn test_index_simple() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "[1, 2, 3][0]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 1);
}

#[test]
fn test_index_last_element() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "[1, 2, 3][2]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 3);
}

#[test]
fn test_index_with_variable() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "arr[i] where { arr = [10, 20, 30], i = 1 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 20);
}

#[test]
fn test_index_with_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "[5, 10, 15][1 + 1]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 15);
}

#[test]
fn test_index_out_of_bounds_positive() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "[1, 2][5]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]);
    assert!(matches!(
        result,
        Err(EvalError::IndexOutOfBounds {
            index: 5,
            len: 2,
            ..
        })
    ));
}

#[test]
fn test_index_out_of_bounds_negative() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "[1, 2][-1]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]);
    assert!(matches!(
        result,
        Err(EvalError::IndexOutOfBounds {
            index: -1,
            len: 2,
            ..
        })
    ));
}

#[test]
fn test_index_nested_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "[[1, 2], [3, 4]][1][0]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 3);
}

#[test]
fn test_index_float_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "[3.14, 2.71, 1.41][1]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert!((result.as_float().unwrap() - 2.71).abs() < 0.001);
}

#[test]
fn test_index_string_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, r#"["a", "b", "c"][2]"#).unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_str().unwrap(), "c");
}

#[test]
fn test_index_bool_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed = parser::parse(&arena, "[true, false, true][1]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_index_with_where_binding() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let parsed =
        parser::parse(&arena, "arr[idx] where { arr = [100, 200, 300], idx = 2 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let result = eval(type_manager, &arena, &typed, &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 300);
}
