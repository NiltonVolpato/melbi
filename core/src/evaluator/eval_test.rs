//! Unit tests for the evaluator.

use super::*;
use crate::{
    analyzer,
    evaluator::{ResourceExceeded, RuntimeError},
    parser,
    types::manager::TypeManager,
    values::{dynamic::Value, function::NativeFunction},
};
use bumpalo::Bump;

struct Runner<'a> {
    arena: &'a Bump,
    type_mgr: &'a TypeManager<'a>,
}

impl<'a> Runner<'a> {
    fn new(arena: &'a Bump) -> Self {
        Self {
            arena,
            type_mgr: TypeManager::new(arena),
        }
    }
    fn run<'i>(
        &self,
        input: &'i str,
        globals: &[(&'a str, Value<'a, 'a>)],
        arguments: &[(&'a str, Value<'a, 'a>)],
    ) -> Result<Value<'a, 'a>, EvalError> {
        let input = self.arena.alloc_str(input);

        // Derive analyzer global types from evaluator global values.
        let global_types: alloc::vec::Vec<(&str, &crate::types::Type)> = globals
            .iter()
            .map(|(name, value)| (*name, value.ty))
            .collect();

        // Derive analyzer argument types from evaluator argument values.
        let argument_types: alloc::vec::Vec<(&str, &crate::types::Type)> = arguments
            .iter()
            .map(|(name, value)| (*name, value.ty))
            .collect();

        let parsed = parser::parse(self.arena, input).expect("parsing failed");
        let typed = analyzer::analyze(
            self.type_mgr,
            self.arena,
            &parsed,
            &global_types,
            &argument_types,
        )
        .expect("type checking failed");

        eval(self.arena, self.type_mgr, &typed, globals, arguments)
    }

    fn run_with_limits<'i>(
        &self,
        input: &'i str,
        globals: &[(&'a str, Value<'a, 'a>)],
        arguments: &[(&'a str, Value<'a, 'a>)],
        max_stack_depth: usize,
    ) -> Result<Value<'a, 'a>, EvalError> {
        let input = self.arena.alloc_str(input);

        // Derive analyzer global types from evaluator global values.
        let global_types: alloc::vec::Vec<(&str, &crate::types::Type)> = globals
            .iter()
            .map(|(name, value)| (*name, value.ty))
            .collect();

        // Derive analyzer argument types from evaluator argument values.
        let argument_types: alloc::vec::Vec<(&str, &crate::types::Type)> = arguments
            .iter()
            .map(|(name, value)| (*name, value.ty))
            .collect();

        let parsed = parser::parse(self.arena, input).expect("parsing failed");
        let typed = analyzer::analyze(
            self.type_mgr,
            self.arena,
            &parsed,
            &global_types,
            &argument_types,
        )
        .expect("type checking failed");

        eval_with_limits(
            self.arena,
            self.type_mgr,
            &typed,
            globals,
            arguments,
            max_stack_depth,
        )
    }
}

// ============================================================================
// Constants
// ============================================================================

#[test]
fn test_constant_int() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("42", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_constant_negative_int() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-42", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -42);
}

#[test]
fn test_constant_float() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("3.14", &[], &[]).unwrap();
    assert_eq!(result.as_float().unwrap(), 3.14);
}

#[test]
fn test_constant_bool_true() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("true", &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_constant_bool_false() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("false", &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_constant_string() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run(r#""hello""#, &[], &[]).unwrap();
    assert_eq!(result.as_str().unwrap(), "hello");
}

// ============================================================================
// Integer Arithmetic
// ============================================================================

#[test]
fn test_int_addition() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("2 + 3", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn test_int_subtraction() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("10 - 4", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 6);
}

#[test]
fn test_int_multiplication() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("3 * 4", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 12);
}

#[test]
fn test_int_division() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("10 / 2", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn test_int_division_truncates() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("7 / 3", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn test_int_power() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("2 ^ 10", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 1024);
}

#[test]
fn test_int_power_zero() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("5 ^ 0", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 1);
}

#[test]
fn test_int_division_by_zero() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("10 / 0", &[], &[]);
    assert!(matches!(
        result,
        Err(EvalError::Runtime(RuntimeError::DivisionByZero { .. }))
    ));
}

#[test]
fn test_int_wrapping_overflow_add() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("9223372036854775807 + 1", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), i64::MIN);
}

#[test]
fn test_int_wrapping_overflow_mul() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("9223372036854775807 * 2", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), -2);
}

// ============================================================================
// Float Arithmetic
// ============================================================================

#[test]
fn test_float_addition() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("3.14 + 2.0", &[], &[]).unwrap();
    assert!((result.as_float().unwrap() - 5.14).abs() < 0.0001);
}

#[test]
fn test_float_subtraction() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("10.5 - 3.5", &[], &[]).unwrap();
    assert_eq!(result.as_float().unwrap(), 7.0);
}

#[test]
fn test_float_multiplication() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("2.5 * 4.0", &[], &[]).unwrap();
    assert_eq!(result.as_float().unwrap(), 10.0);
}

#[test]
fn test_float_division() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("10.0 / 3.0", &[], &[]).unwrap();
    let expected = 10.0 / 3.0;
    assert!((result.as_float().unwrap() - expected).abs() < 0.0001);
}

#[test]
fn test_float_division_by_zero() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("10.0 / 0.0", &[], &[]).unwrap();
    assert!(result.as_float().unwrap().is_infinite());
}

#[test]
fn test_float_power() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("2.0 ^ 3.0", &[], &[]).unwrap();
    assert_eq!(result.as_float().unwrap(), 8.0);
}

// ============================================================================
// Boolean Operators (Milestone 1.3) - Short-Circuit Evaluation
// ============================================================================

#[test]
fn test_boolean_and_true_true() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("true and true", &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_boolean_and_true_false() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("true and false", &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_boolean_and_false_true() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("false and true", &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_boolean_and_false_false() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("false and false", &[], &[])
        .unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_boolean_or_true_true() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("true or true", &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_boolean_or_true_false() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("true or false", &[], &[]).unwrap();
    // Right side not evaluated due to short-circuit
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_boolean_or_false_true() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("false or true", &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_boolean_or_false_false() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("false or false", &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_boolean_chain_and() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("true and true and false", &[], &[])
        .unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_boolean_chain_or() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("false or false or true", &[], &[])
        .unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_boolean_mixed_chain() {
    let arena = Bump::new();
    // 'and' has higher precedence than 'or'
    // So: true and false or true = (true and false) or true = false or true = true
    let result = Runner::new(&arena)
        .run("true and false or true", &[], &[])
        .unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_boolean_with_variables() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [
        ("x", Value::bool(runner.type_mgr, true)),
        ("y", Value::bool(runner.type_mgr, false)),
    ];
    let result = runner.run("x and y", &[], &var_values).unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_boolean_short_circuit_and_with_where() {
    let arena = Bump::new();
    // false and (x where { x = true })
    // The where expression should not be evaluated due to short-circuit
    let result = Runner::new(&arena)
        .run("false and (x where { x = true })", &[], &[])
        .unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_boolean_short_circuit_or_with_where() {
    let arena = Bump::new();
    // true or (x where { x = false })
    // The where expression should not be evaluated due to short-circuit
    let result = Runner::new(&arena)
        .run("true or (x where { x = false })", &[], &[])
        .unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

// ============================================================================
// Nested Expressions
// ============================================================================

#[test]
fn test_nested_arithmetic() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("(2 + 3) * 4", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 20);
}

#[test]
fn test_deeply_nested() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("((1 + 2) * (3 + 4)) - (5 * 6)", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), (1 + 2) * (3 + 4) - (5 * 6));
}

#[test]
fn test_operator_precedence() {
    let arena = Bump::new();

    // Verify that * binds tighter than +
    let result = Runner::new(&arena).run("2 + 3 * 4", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 14); // Not 20

    // Verify that ^ binds tighter than *
    let result = Runner::new(&arena).run("2 * 3 ^ 2", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 18); // Not 36
}

// ============================================================================
// Stack Depth Limit
// ============================================================================

#[test]
fn test_stack_depth_limit() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Create a deeply nested expression using actual operations: 1 + (1 + (1 + ...))
    // This creates real recursion depth, unlike just parentheses
    let mut source = String::from("1");
    for _ in 0..100 {
        source = format!("1 + ({})", source);
    }

    // With default limit of 1000, this should succeed (100 < 1000)
    let result = runner.run(&source, &[], &[]);
    assert!(result.is_ok());

    // But with a lower limit of 50, it should fail
    let result = runner.run_with_limits(&source, &[], &[], 50);
    assert!(matches!(
        result,
        Err(EvalError::ResourceExceeded(
            ResourceExceeded::StackOverflow { .. }
        ))
    ));
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
    let result = eval_with_limits(&arena, type_manager, &typed, &[], &[], 100);
    assert!(result.is_ok());

    // But with limit of 40, it should fail
    let result = eval_with_limits(&arena, type_manager, &typed, &[], &[], 40);
    assert!(matches!(
        result,
        Err(EvalError::ResourceExceeded(
            ResourceExceeded::StackOverflow { .. }
        ))
    ));
}

// ============================================================================
// Variables (Runtime Parameters)
// ============================================================================

#[test]
fn test_variable_simple_lookup() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [("x", Value::int(runner.type_mgr, 42))];
    let result = runner.run("x", &[], &var_values).unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_variable_in_expression() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [("x", Value::int(runner.type_mgr, 5))];
    let result = runner.run("x * 2 + 10", &[], &var_values).unwrap();
    assert_eq!(result.as_int().unwrap(), 20); // 5 * 2 + 10 = 20
}

#[test]
fn test_multiple_variables() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [
        ("x", Value::int(runner.type_mgr, 10)),
        ("y", Value::int(runner.type_mgr, 20)),
    ];
    let result = runner.run("x + y * 2", &[], &var_values).unwrap();
    assert_eq!(result.as_int().unwrap(), 50); // 10 + 20 * 2 = 50
}

#[test]
fn test_variable_float() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [("price", Value::float(runner.type_mgr, 100.0))];
    let result = runner.run("price * 1.2", &[], &var_values).unwrap();
    assert_eq!(result.as_float().unwrap(), 120.0);
}

#[test]
fn test_variable_bool() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [("flag", Value::bool(runner.type_mgr, true))];
    let result = runner.run("flag", &[], &var_values).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_variable_string() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [("name", Value::str(&arena, runner.type_mgr.str(), "Alice"))];
    let result = runner.run("name", &[], &var_values).unwrap();
    assert_eq!(result.as_str().unwrap(), "Alice");
}

// ============================================================================
// Globals (Constants and Built-in Functions)
// ============================================================================

#[test]
fn test_global_constant_pi() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let globals_values = [("PI", Value::float(runner.type_mgr, 3.14159))];
    let result = runner.run("PI * 2.0", &globals_values, &[]).unwrap();
    assert!((result.as_float().unwrap() - 6.28318).abs() < 0.0001);
}

#[test]
fn test_global_constant_with_variables() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let globals_values = [("PI", Value::float(runner.type_mgr, 3.14159))];
    let var_values = [("radius", Value::float(runner.type_mgr, 5.0))];
    let result = runner
        .run("PI * radius * radius", &globals_values, &var_values)
        .unwrap();

    // Area = PI * r^2 = 3.14159 * 5 * 5 = 78.53975
    assert!((result.as_float().unwrap() - 78.53975).abs() < 0.001);
}

#[test]
fn test_multiple_globals() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let globals_values = [
        ("E", Value::float(runner.type_mgr, 2.71828)),
        ("PI", Value::float(runner.type_mgr, 3.14159)),
    ];
    let result = runner.run("PI + E", &globals_values, &[]).unwrap();
    assert!((result.as_float().unwrap() - 5.85987).abs() < 0.0001);
}

// ============================================================================
// Shadowing Tests (Variables vs Where Bindings)
// ============================================================================

#[test]
fn test_where_shadows_variable() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [("x", Value::int(runner.type_mgr, 10))];
    let result = runner.run("x where { x = 5 }", &[], &var_values).unwrap();

    // Inner x = 5 shadows outer x = 10
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn test_where_can_reference_variable() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [("x", Value::int(runner.type_mgr, 10))];
    let result = runner
        .run("y where { y = x * 2 }", &[], &var_values)
        .unwrap();

    // y = x * 2 = 10 * 2 = 20
    assert_eq!(result.as_int().unwrap(), 20);
}

// ============================================================================
// Where Expressions (Local Scoping)
// ============================================================================

#[test]
fn test_where_simple() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("x where { x = 42 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_where_multiple_bindings() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("x + y where { x = 10, y = 20 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 30);
}

#[test]
fn test_where_sequential_binding() {
    let arena = Bump::new();
    // b can reference a (sequential binding)
    let result = Runner::new(&arena)
        .run("b where { a = 1, b = a + 1 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn test_where_sequential_binding_chain() {
    let arena = Bump::new();
    // c can reference b which references a
    let result = Runner::new(&arena)
        .run("c where { a = 1, b = a * 2, c = b + 1 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 3); // a=1, b=2, c=3
}

#[test]
fn test_where_complex_expression() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("a + b + c where { a = 1, b = a * 2, c = b + 1 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 6); // 1 + 2 + 3 = 6
}

#[test]
fn test_where_nested_scopes() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("x + y where { x = 10 } where { y = 20 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 30);
}

#[test]
fn test_where_with_arithmetic() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("(a + b) * c where { a = 2, b = 3, c = 4 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 20); // (2 + 3) * 4 = 20
}

#[test]
fn test_where_with_float() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("x * y where { x = 2.5, y = 4.0 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_float().unwrap(), 10.0);
}

// ============================================================================
// Records (Milestone 2.2)
// ============================================================================

#[test]
fn test_record_empty() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("Record{}", &[], &[]).unwrap();
    let record = result.as_record().unwrap();
    assert_eq!(record.len(), 0);
    assert_eq!(format!("{}", result), "{}");
}

#[test]
fn test_record_simple() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("{ x = 42, y = 3.14 }", &[], &[])
        .unwrap();
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
    let result = Runner::new(&arena).run("{ x = 42 }.x", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_field_access_multiple_fields() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("{ a = 10, b = 20, c = 30 }.b", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 20);
}

#[test]
fn test_field_access_in_expression() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("{ x = 5, y = 10 }.x + { x = 5, y = 10 }.y", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 15);
}

#[test]
fn test_record_with_where() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("{ x = a, y = b } where { a = 1, b = 2 }", &[], &[])
        .unwrap();
    let record = result.as_record().unwrap();

    let x = record.get("x").unwrap();
    assert_eq!(x.as_int().unwrap(), 1);

    let y = record.get("y").unwrap();
    assert_eq!(y.as_int().unwrap(), 2);
}

#[test]
fn test_nested_record() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "{ point = { x = 10, y = 20 }, name = \"origin\" }",
            &[],
            &[],
        )
        .unwrap();
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
    let result = Runner::new(&arena)
        .run("{ point = { x = 10, y = 20 } }.point.x", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 10);
}

#[test]
fn test_math_package_record() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Create Math record type with PI and E fields
    let math_ty = runner.type_mgr.record(vec![
        ("E", runner.type_mgr.float()),
        ("PI", runner.type_mgr.float()),
    ]);

    // Create Math record value with PI and E
    let math_value = Value::record(
        &arena,
        math_ty,
        &[
            ("E", Value::float(runner.type_mgr, 2.71828)),
            ("PI", Value::float(runner.type_mgr, 3.14159)),
        ],
    )
    .unwrap();

    let globals_values = [("Math", math_value)];
    let result = runner
        .run("Math.PI * 2.0 + Math.E", &globals_values, &[])
        .unwrap();

    // Math.PI * 2.0 + Math.E = 3.14159 * 2.0 + 2.71828 = 6.28318 + 2.71828 = 9.00146
    assert!((result.as_float().unwrap() - 9.00146).abs() < 0.0001);
}

#[test]
fn test_math_package_circle_area() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Create Math record type
    let math_ty = runner
        .type_mgr
        .record(vec![("PI", runner.type_mgr.float())]);

    // Create Math record value
    let math_value = Value::record(
        &arena,
        math_ty,
        &[("PI", Value::float(runner.type_mgr, 3.14159))],
    )
    .unwrap();

    let globals_values = [("Math", math_value)];
    let var_values = [("radius", Value::float(runner.type_mgr, 5.0))];
    let result = runner
        .run("Math.PI * radius * radius", &globals_values, &var_values)
        .unwrap();

    // Area = Math.PI * r^2 = 3.14159 * 5 * 5 = 78.53975
    assert!((result.as_float().unwrap() - 78.53975).abs() < 0.001);
}

// ================================
// Unary Operator Tests
// ================================

#[test]
fn test_unary_negation_int() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-42", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -42);
}

#[test]
fn test_unary_negation_int_positive() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-(42)", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -42);
}

#[test]
fn test_unary_double_negation() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-(-5)", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn test_unary_negation_expression() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-(1 + 2)", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -3);
}

#[test]
fn test_unary_negation_float() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-(3.14)", &[], &[]).unwrap();
    assert!((result.as_float().unwrap() + 3.14).abs() < 0.0001);
}

#[test]
fn test_unary_negation_float_expression() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-(2.5 + 1.5)", &[], &[]).unwrap();
    assert!((result.as_float().unwrap() + 4.0).abs() < 0.0001);
}

#[test]
fn test_unary_not_true() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("not true", &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_unary_not_false() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("not false", &[], &[]).unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_unary_not_expression() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("not (true and false)", &[], &[])
        .unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_unary_with_where() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("-x where { x = 42 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), -42);
}

#[test]
fn test_unary_negation_wrapping() {
    let arena = Bump::new();
    // Use string interpolation to build the source with i64::MIN
    let source = format!("-({})", i64::MIN);
    let result = Runner::new(&arena).run(&source, &[], &[]).unwrap();
    // -i64::MIN wraps to i64::MIN
    assert_eq!(result.as_int().unwrap(), i64::MIN);
}

// ================================
// If/Else Expression Tests
// ================================

#[test]
fn test_if_true_branch() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if true then 1 else 2", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 1);
}

#[test]
fn test_if_false_branch() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if false then 1 else 2", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn test_if_with_variable() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Test with flag = true
    let var_values = [("flag", Value::bool(runner.type_mgr, true))];
    let result = runner
        .run("if flag then 10 else 20", &[], &var_values)
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 10);

    // Test with flag = false
    let var_values = [("flag", Value::bool(runner.type_mgr, false))];
    let result = runner
        .run("if flag then 10 else 20", &[], &var_values)
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 20);
}

#[test]
fn test_if_with_expression_condition() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if true and false then 1 else 2", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn test_if_with_where() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if x then 1 else 2 where { x = true }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 1);
}

#[test]
fn test_if_nested() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if true then (if false then 1 else 2) else 3", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn test_if_float_branches() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if true then 3.14 else 2.71", &[], &[])
        .unwrap();
    assert!((result.as_float().unwrap() - 3.14).abs() < 0.0001);
}

#[test]
fn test_if_string_branches() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"if false then "yes" else "no""#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "no");
}

#[test]
fn test_if_bool_branches() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if true then true else false", &[], &[])
        .unwrap();
    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_if_with_complex_expressions() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if true then (1 + 2) * 3 else 4 ^ 2", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 9);
}

// ================================
// Array Tests
// ================================

#[test]
fn test_array_empty() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("[]", &[], &[]).unwrap();

    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 0);
}

#[test]
fn test_array_simple_int() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("[1, 2, 3]", &[], &[]).unwrap();

    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_int().unwrap(), 1);
    assert_eq!(array.get(1).unwrap().as_int().unwrap(), 2);
    assert_eq!(array.get(2).unwrap().as_int().unwrap(), 3);
}

#[test]
fn test_array_simple_float() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[3.14, 2.71, 1.41]", &[], &[])
        .unwrap();

    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert!((array.get(0).unwrap().as_float().unwrap() - 3.14).abs() < 0.001);
    assert!((array.get(1).unwrap().as_float().unwrap() - 2.71).abs() < 0.001);
    assert!((array.get(2).unwrap().as_float().unwrap() - 1.41).abs() < 0.001);
}

#[test]
fn test_array_simple_bool() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[true, false, true]", &[], &[])
        .unwrap();

    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_bool().unwrap(), true);
    assert_eq!(array.get(1).unwrap().as_bool().unwrap(), false);
    assert_eq!(array.get(2).unwrap().as_bool().unwrap(), true);
}

#[test]
fn test_array_simple_string() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"["a", "b", "c"]"#, &[], &[])
        .unwrap();

    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_str().unwrap(), "a");
    assert_eq!(array.get(1).unwrap().as_str().unwrap(), "b");
    assert_eq!(array.get(2).unwrap().as_str().unwrap(), "c");
}

#[test]
fn test_array_with_expressions() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[1 + 1, 2 * 2, 3 ^ 2]", &[], &[])
        .unwrap();

    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_int().unwrap(), 2);
    assert_eq!(array.get(1).unwrap().as_int().unwrap(), 4);
    assert_eq!(array.get(2).unwrap().as_int().unwrap(), 9);
}

#[test]
fn test_array_nested() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[[1, 2], [3, 4]]", &[], &[])
        .unwrap();

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
    let result = Runner::new(&arena)
        .run("[x, y, z] where { x = 1, y = 2, z = 3 }", &[], &[])
        .unwrap();

    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_int().unwrap(), 1);
    assert_eq!(array.get(1).unwrap().as_int().unwrap(), 2);
    assert_eq!(array.get(2).unwrap().as_int().unwrap(), 3);
}

#[test]
fn test_array_with_variables() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [
        ("x", Value::int(runner.type_mgr, 10)),
        ("y", Value::int(runner.type_mgr, 20)),
        ("z", Value::int(runner.type_mgr, 30)),
    ];
    let result = runner.run("[x, y, z]", &[], &var_values).unwrap();
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
    let result = Runner::new(&arena).run("[1, 2, 3][0]", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 1);
}

#[test]
fn test_index_last_element() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("[1, 2, 3][2]", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 3);
}

#[test]
fn test_index_with_variable() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("arr[i] where { arr = [10, 20, 30], i = 1 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 20);
}

#[test]
fn test_index_with_expression() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[5, 10, 15][1 + 1]", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 15);
}

#[test]
fn test_index_out_of_bounds_positive() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("[1, 2][5]", &[], &[]);
    assert!(matches!(
        result,
        Err(EvalError::Runtime(RuntimeError::IndexOutOfBounds {
            index: 5,
            len: 2,
            ..
        }))
    ));
}

#[test]
fn test_index_out_of_bounds_negative() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("[1, 2][-1]", &[], &[]);
    assert!(matches!(
        result,
        Err(EvalError::Runtime(RuntimeError::IndexOutOfBounds {
            index: -1,
            len: 2,
            ..
        }))
    ));
}

#[test]
fn test_index_nested_array() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[[1, 2], [3, 4]][1][0]", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 3);
}

#[test]
fn test_index_float_array() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[3.14, 2.71, 1.41][1]", &[], &[])
        .unwrap();
    assert!((result.as_float().unwrap() - 2.71).abs() < 0.001);
}

#[test]
fn test_index_string_array() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"["a", "b", "c"][2]"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "c");
}

#[test]
fn test_index_bool_array() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[true, false, true][1]", &[], &[])
        .unwrap();
    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_index_with_where_binding() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "arr[idx] where { arr = [100, 200, 300], idx = 2 }",
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 300);
}

// ================================
// Format String Tests
// ================================

#[test]
fn test_format_str_no_interpolation() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"hello world""#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "hello world");
}

#[test]
fn test_format_str_single_int() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"x = {x}" where { x = 42 }"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "x = 42");
}

#[test]
fn test_format_str_multiple_values() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"{a} + {b} = {a + b}" where { a = 1, b = 2 }"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "1 + 2 = 3");
}

#[test]
fn test_format_str_with_string() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"Hello, {name}!" where { name = "World" }"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "Hello, World!");
}

#[test]
fn test_format_str_with_float() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"Pi = {pi}" where { pi = 3.14 }"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "Pi = 3.14");
}

#[test]
fn test_format_str_with_bool() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"Flag: {flag}" where { flag = true }"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "Flag: true");
}

#[test]
fn test_format_str_with_array() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"Array: {arr}" where { arr = [1, 2, 3] }"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "Array: [1, 2, 3]");
}

#[test]
fn test_format_str_consecutive_expressions() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"{x}{y}" where { x = 1, y = 2 }"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "12");
}

#[test]
fn test_format_str_mixed_types() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            r#"f"Int: {i}, Float: {f}, Bool: {b}" where { i = 42, f = 3.14, b = true }"#,
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "Int: 42, Float: 3.14, Bool: true");
}

#[test]
fn test_format_str_with_variables() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [
        ("age", Value::int(runner.type_mgr, 30)),
        ("name", Value::str(&arena, runner.type_mgr.str(), "Alice")),
    ];
    let result = runner
        .run(r#"f"{name} is {age} years old""#, &[], &var_values)
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "Alice is 30 years old");
}

#[test]
fn test_format_str_string_no_quotes() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"Result: {s}" where { s = "test" }"#, &[], &[])
        .unwrap();
    // String should NOT have quotes in the output
    assert_eq!(result.as_str().unwrap(), "Result: test");
}

#[test]
fn test_format_str_array_with_strings() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            r#"f"Items: {items}" where { items = ["a", "b", "c"] }"#,
            &[],
            &[],
        )
        .unwrap();
    // Array uses Debug, so strings inside should have quotes
    assert_eq!(result.as_str().unwrap(), r#"Items: ["a", "b", "c"]"#);
}

// ================================
// Otherwise Operator Tests
// ================================

#[test]
fn test_otherwise_no_error() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("(10 / 2) otherwise -1", &[], &[])
        .unwrap();

    // Primary succeeds, return its value
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn test_otherwise_division_by_zero() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("(10 / 0) otherwise -1", &[], &[])
        .unwrap();

    // Primary fails (division by zero), return fallback
    assert_eq!(result.as_int().unwrap(), -1);
}

#[test]
fn test_otherwise_index_out_of_bounds() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[1, 2][5] otherwise -1", &[], &[])
        .unwrap();

    // Primary fails (index out of bounds), return fallback
    assert_eq!(result.as_int().unwrap(), -1);
}

#[test]
fn test_otherwise_negative_index() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[1, 2][-1] otherwise 99", &[], &[])
        .unwrap();
    // Primary fails (negative index), return fallback
    assert_eq!(result.as_int().unwrap(), 99);
}

#[test]
fn test_otherwise_with_variables() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Test with valid index
    let arr_value = Value::array(
        &arena,
        runner.type_mgr.array(runner.type_mgr.int()),
        &[
            Value::int(runner.type_mgr, 10),
            Value::int(runner.type_mgr, 20),
            Value::int(runner.type_mgr, 30),
        ],
    )
    .unwrap();
    let var_values = [
        ("arr", arr_value),
        ("default", Value::int(runner.type_mgr, -1)),
        ("idx", Value::int(runner.type_mgr, 1)),
    ];
    let result = runner
        .run("arr[idx] otherwise default", &[], &var_values)
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 20);

    // Test with invalid index
    let var_values = [
        ("arr", arr_value),
        ("default", Value::int(runner.type_mgr, -1)),
        ("idx", Value::int(runner.type_mgr, 10)),
    ];
    let result = runner
        .run("arr[idx] otherwise default", &[], &var_values)
        .unwrap();
    assert_eq!(result.as_int().unwrap(), -1);
}

#[test]
fn test_otherwise_nested() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("(10 / 0) otherwise ((5 / 0) otherwise 42)", &[], &[])
        .unwrap();
    // Both primary and first fallback fail, return nested fallback
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_otherwise_with_where() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "(arr[i] otherwise def) where { arr = [1, 2], i = 5, def = 99 }",
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 99);
}

#[test]
fn test_otherwise_fallback_expression() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("(10 / 0) otherwise (2 + 3)", &[], &[])
        .unwrap();
    // Primary fails, evaluate fallback expression
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn test_otherwise_string_type() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"["a", "b"][10] otherwise "default""#, &[], &[])
        .unwrap();
    // Index out of bounds, return fallback
    assert_eq!(result.as_str().unwrap(), "default");
}

#[test]
fn test_otherwise_float_type() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("(1.0 / 0.0) otherwise 3.14", &[], &[])
        .unwrap();
    // Float division by zero produces inf, not an error
    // So this should return the primary result (inf)
    assert!(result.as_float().unwrap().is_infinite());
}

#[test]
fn test_otherwise_does_not_catch_stack_overflow() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Create a deeply nested expression that will exceed stack depth
    // Use a very small depth limit to trigger overflow quickly
    let mut expr = "1".to_string();
    for _ in 0..50 {
        expr = format!("({}) + 1", expr);
    }

    // Add otherwise clause - this should NOT catch the StackOverflow error
    let source = format!("({}) otherwise 999", expr);

    let parsed = parser::parse(&arena, &source).unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    // Use a very small depth limit to trigger stack overflow
    let result = eval_with_limits(&arena, type_manager, &typed, &[], &[], 10);

    // Should get StackOverflow error, NOT the fallback value
    match result {
        Err(EvalError::ResourceExceeded(ResourceExceeded::StackOverflow { .. })) => {
            // Got the expected error - otherwise did not catch it
        }
        Ok(_) => panic!("Expected StackOverflow error, but evaluation succeeded"),
        Err(e) => panic!("Expected StackOverflow error, got: {:?}", e),
    }
}

// ============================================================================
// Cast Tests
// ============================================================================

#[test]
fn test_cast_int_to_float() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("42 as Float", &[], &[]).unwrap();
    assert_eq!(result.as_float().unwrap(), 42.0);
}

#[test]
fn test_cast_float_to_int_truncates() {
    let arena = Bump::new();
    // Positive truncation
    let result = Runner::new(&arena).run("3.7 as Int", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 3);
    // Negative truncation
    let result = Runner::new(&arena).run("(-3.7) as Int", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -3);
}

#[test]
fn test_cast_str_to_bytes() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#""hello" as Bytes"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_bytes().unwrap(), b"hello");
}

#[test]
fn test_cast_bytes_to_str_valid_utf8() {
    let arena = Bump::new();
    // First create bytes, then cast back to string
    let result = Runner::new(&arena)
        .run(r#"("hello" as Bytes) as String"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "hello");
}

#[test]
fn test_cast_bytes_to_str_invalid_utf8() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Create invalid UTF-8 bytes via variable
    let invalid_bytes = &[0xFF, 0xFE, 0xFD];
    let bytes_value = Value::bytes(&arena, runner.type_mgr.bytes(), invalid_bytes);

    let var_values = &[("invalid", bytes_value)];
    let result = runner.run("invalid as String", &[], var_values);

    // Should fail with CastError
    assert!(result.is_err());
    match result {
        Err(EvalError::Runtime(RuntimeError::CastError { .. })) => {
            // Expected
        }
        _ => panic!("Expected CastError"),
    }
}

#[test]
fn test_cast_with_otherwise() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Create invalid UTF-8 bytes via variable
    let invalid_bytes = &[0xFF, 0xFE, 0xFD];
    let bytes_value = Value::bytes(&arena, runner.type_mgr.bytes(), invalid_bytes);

    let var_values = &[("data", bytes_value)];

    // Use otherwise to handle invalid UTF-8
    let result = runner
        .run(r#"(data as String) otherwise "fallback""#, &[], var_values)
        .unwrap();

    // Should get the fallback value
    assert_eq!(result.as_str().unwrap(), "fallback");
}

#[test]
fn test_cast_in_expression() {
    let arena = Bump::new();
    // Cast within arithmetic expression
    let result = Runner::new(&arena)
        .run("(42 as Float) + 0.5", &[], &[])
        .unwrap();
    assert_eq!(result.as_float().unwrap(), 42.5);
}

#[test]
fn test_cast_with_where() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("(x as Float) * 2.0 where { x = 21 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_float().unwrap(), 42.0);
}

#[test]
fn test_cast_utf8_roundtrip() {
    let arena = Bump::new();
    // String → Bytes → String should preserve unicode
    let result = Runner::new(&arena)
        .run(r#"(("Hello, 世界! 🦀" as Bytes) as String)"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "Hello, 世界! 🦀");
}

// ============================================================================
// FFI Function Calls
// ============================================================================

// Test FFI functions

fn ffi_add<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, EvalError> {
    assert_eq!(args.len(), 2);
    let a = args[0].as_int().unwrap();
    let b = args[1].as_int().unwrap();
    Ok(Value::int(type_mgr, a + b))
}

fn ffi_concat<'types, 'arena>(
    arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, EvalError> {
    assert_eq!(args.len(), 2);
    let a = args[0].as_str().unwrap();
    let b = args[1].as_str().unwrap();
    let result = arena.alloc_str(&format!("{}{}", a, b));
    Ok(Value::str(arena, type_mgr.str(), result))
}

fn ffi_array_len<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, EvalError> {
    assert_eq!(args.len(), 1);
    let array = args[0].as_array().unwrap();
    Ok(Value::int(type_mgr, array.len() as i64))
}

fn ffi_divide<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, EvalError> {
    assert_eq!(args.len(), 2);
    let a = args[0].as_int().unwrap();
    let b = args[1].as_int().unwrap();
    if b == 0 {
        return Err(RuntimeError::DivisionByZero { span: None }.into());
    }
    Ok(Value::int(type_mgr, a / b))
}

#[test]
fn test_ffi_simple_call() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let add_ty = runner.type_mgr.function(
        &[runner.type_mgr.int(), runner.type_mgr.int()],
        runner.type_mgr.int(),
    );
    let add_fn = Value::function(&arena, add_ty, NativeFunction(ffi_add)).unwrap();

    let result = runner.run("add(10, 32)", &[("add", add_fn)], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_ffi_nested_calls() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let add_ty = runner.type_mgr.function(
        &[runner.type_mgr.int(), runner.type_mgr.int()],
        runner.type_mgr.int(),
    );
    let add_fn = Value::function(&arena, add_ty, NativeFunction(ffi_add)).unwrap();

    let result = runner
        .run("add(add(1, 2), 3)", &[("add", add_fn)], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 6);
}

#[test]
fn test_ffi_string_concat() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let concat_ty = runner.type_mgr.function(
        &[runner.type_mgr.str(), runner.type_mgr.str()],
        runner.type_mgr.str(),
    );
    let concat_fn = Value::function(&arena, concat_ty, NativeFunction(ffi_concat)).unwrap();

    let result = runner
        .run(r#"concat("hello", "world")"#, &[("concat", concat_fn)], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "helloworld");
}

#[test]
fn test_ffi_polymorphic_array_len() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // len : Array[T] -> Int (polymorphic)
    let len_ty = {
        let t_var = runner.type_mgr.fresh_type_var();
        let array_t = runner.type_mgr.array(t_var);
        runner.type_mgr.function(&[array_t], runner.type_mgr.int())
    };
    let len_fn = Value::function(&arena, len_ty, NativeFunction(ffi_array_len)).unwrap();

    let result = runner
        .run("len([1, 2, 3])", &[("len", len_fn)], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 3);
}

#[test]
fn test_ffi_error_with_otherwise() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let divide_ty = runner.type_mgr.function(
        &[runner.type_mgr.int(), runner.type_mgr.int()],
        runner.type_mgr.int(),
    );
    let divide_fn = Value::function(&arena, divide_ty, NativeFunction(ffi_divide)).unwrap();

    let result = runner
        .run("divide(10, 0) otherwise -1", &[("divide", divide_fn)], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), -1);
}

#[test]
fn test_ffi_call_with_variables() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let add_ty = runner.type_mgr.function(
        &[runner.type_mgr.int(), runner.type_mgr.int()],
        runner.type_mgr.int(),
    );
    let add_fn = Value::function(&arena, add_ty, NativeFunction(ffi_add)).unwrap();

    let result = runner
        .run(
            "add(x, y) where { x = 10, y = 32 }",
            &[("add", add_fn)],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}
