use super::*;
use crate::format;
use crate::{parser, types::manager::TypeManager};
use bumpalo::Bump;

// Helper to parse and analyze a source string
fn analyze_source<'types, 'arena>(
    source: &'arena str,
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
) -> Result<&'arena typed_expr::TypedExpr<'types, 'arena>, miette::Report>
where
    'types: 'arena,
{
    let parsed = parser::parse(arena, source)
        .map_err(|e| miette::Report::msg(format!("Parse error: {:?}", e)))?;
    analyze(type_manager, arena, &parsed)
}

// ============================================================================
// Binary Operations
// ============================================================================

#[test]
fn test_arithmetic_operators_integers() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    for op in ["+", "-", "*", "/", "^"] {
        let source = format!("1 {} 2", op);
        let result = analyze_source(&source, &type_manager, &bump);
        assert!(result.is_ok(), "Failed for operator {}", op);
        assert_eq!(result.unwrap().expr.0, type_manager.int());
    }
}

#[test]
fn test_arithmetic_operators_floats() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    for op in ["+", "-", "*", "/", "^"] {
        let source = format!("1.0 {} 2.0", op);
        let result = analyze_source(&source, &type_manager, &bump);
        assert!(result.is_ok(), "Failed for operator {}", op);
        assert_eq!(result.unwrap().expr.0, type_manager.float());
    }
}

#[test]
fn test_arithmetic_mixed_types_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("1 + 2.0", &type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn test_logical_operators() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    for op in ["and", "or"] {
        let source = format!("true {} false", op);
        let result = analyze_source(&source, &type_manager, &bump);
        assert!(result.is_ok(), "Failed for operator {}", op);
        assert_eq!(result.unwrap().expr.0, type_manager.bool());
    }
}

#[test]
fn test_logical_operators_non_boolean_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("1 and 2", &type_manager, &bump);
    assert!(result.is_err());
}

// ============================================================================
// Unary Operations
// ============================================================================

#[test]
fn test_unary_negation() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Note: -42 is a negative literal, not negation. Need -(42) to test the operator
    let result = analyze_source("-(42)", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());

    let result = analyze_source("-(3.14)", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.float());

    // Also test with a variable to ensure it works on non-literals
    let result = analyze_source("-x where { x = 5 }", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn test_unary_negation_non_numeric_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("-(true)", &type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn test_unary_not() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("not true", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.bool());
}

#[test]
fn test_unary_not_non_boolean_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("not 42", &type_manager, &bump);
    assert!(result.is_err());
}

// ============================================================================
// Literals
// ============================================================================

#[test]
fn test_literals() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let int_result = analyze_source("42", &type_manager, &bump);
    assert!(int_result.is_ok());
    assert_eq!(int_result.unwrap().expr.0, type_manager.int());

    let float_result = analyze_source("3.14", &type_manager, &bump);
    assert!(float_result.is_ok());
    assert_eq!(float_result.unwrap().expr.0, type_manager.float());

    let bool_result = analyze_source("true", &type_manager, &bump);
    assert!(bool_result.is_ok());
    assert_eq!(bool_result.unwrap().expr.0, type_manager.bool());

    let str_result = analyze_source("\"hello\"", &type_manager, &bump);
    assert!(str_result.is_ok());
    assert_eq!(str_result.unwrap().expr.0, type_manager.str());
}

#[test]
fn test_all_literal_types() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source(
        "{ int = 42, float = 3.14, bool = false, str = \"foo\", bytes = b\"bar\" }",
        &type_manager,
        &bump,
    );
    assert!(result.is_ok());

    let expected_type = type_manager.record(&[
        ("int", type_manager.int()),
        ("float", type_manager.float()),
        ("bool", type_manager.bool()),
        ("str", type_manager.str()),
        ("bytes", type_manager.bytes()),
    ]);

    assert_eq!(result.unwrap().expr.0, expected_type);
}

// ============================================================================
// Cast
// ============================================================================

#[test]
fn test_cast_valid() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("42 as Int", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());

    let result = analyze_source("[1, 2, 3] as Array[Int]", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().expr.0,
        type_manager.array(type_manager.int())
    );
}

#[test]
fn test_cast_unknown_type_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("42 as Foo", &type_manager, &bump);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_string = format!("{:?}", err);
    assert!(err_string.contains("Unknown type: Foo"));
}

// ============================================================================
// Variables and Scopes
// ============================================================================

#[test]
fn test_undefined_variable_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("undefined_var", &type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn test_where_binding() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("x where { x = 42 }", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn test_where_duplicate_binding_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("x where { x = 1, x = 2 }", &type_manager, &bump);
    assert!(result.is_err());
}

// ============================================================================
// Lambdas and Functions
// ============================================================================

#[test]
fn test_lambda_basic() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("(x) => x", &type_manager, &bump);
    assert!(result.is_ok());

    // Check it's a function type
    match result.unwrap().expr.0 {
        crate::types::Type::Function { .. } => {}
        _ => panic!("Expected function type"),
    }
}

#[test]
fn test_lambda_duplicate_parameter_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("(x, x) => x", &type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn test_function_call() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("((x) => x)(42)", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn test_call_non_function_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("42()", &type_manager, &bump);
    assert!(result.is_err());
}

// ============================================================================
// If Expressions
// ============================================================================

#[test]
fn test_if_expression() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("if true then 1 else 2", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn test_if_non_boolean_condition_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("if 1 then 2 else 3", &type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn test_if_mismatched_branches_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("if true then 1 else false", &type_manager, &bump);
    assert!(result.is_err());
}

// ============================================================================
// Arrays
// ============================================================================

#[test]
fn test_array_homogeneous() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("[1, 2, 3]", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().expr.0,
        type_manager.array(type_manager.int())
    );
}

#[test]
fn test_array_heterogeneous_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("[1, true]", &type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn test_array_empty() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("[]", &type_manager, &bump);
    assert!(result.is_ok());

    match result.unwrap().expr.0 {
        crate::types::Type::Array(..) => {}
        _ => panic!("Expected array type"),
    }
}

// ============================================================================
// Incomplete Features (marked with #[ignore])
// ============================================================================

#[test]
fn test_array_indexing() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("[1, 2, 3][0]", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn test_array_indexing_with_variable() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source(
        "arr[i] where { arr = [1, 2, 3], i = 0 }",
        &type_manager,
        &bump,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn test_array_indexing_non_integer_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("[1, 2, 3][true]", &type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn test_indexing_non_indexable_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("42[0]", &type_manager, &bump);
    assert!(result.is_err());
}

// ============================================================================
// Record Tests
// ============================================================================

#[test]
fn test_record_empty() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("Record{}", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.record(&[]));
}

#[test]
fn test_record_single_field() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ x = 42 }", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().expr.0,
        type_manager.record(&[("x", type_manager.int())])
    );
}

#[test]
fn test_record_multiple_fields() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ x = 42, y = true, z = \"hello\" }", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().expr.0,
        type_manager.record(&[
            ("x", type_manager.int()),
            ("y", type_manager.bool()),
            ("z", type_manager.str())
        ])
    );
}

// ============================================================================
// Field Access Tests
// ============================================================================

#[test]
fn test_record_field_access() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ x = 42 }.x", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn test_record_field_access_multiple_fields() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ x = 42, y = \"hello\" }.y", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.str());
}

#[test]
fn test_record_field_access_nonexistent_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ x = 42 }.y", &type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn test_field_access_non_record_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("42.x", &type_manager, &bump);
    assert!(result.is_err());
}

// ============================================================================
// Map Tests
// ============================================================================

#[test]
fn test_map_empty() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{}", &type_manager, &bump);
    assert!(result.is_ok());
    match result.unwrap().expr.0 {
        crate::types::Type::Map(..) => {}
        _ => panic!("Expected map type"),
    }
}

#[test]
fn test_map_homogeneous_types() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ \"a\": 1, \"b\": 2 }", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().expr.0,
        type_manager.map(type_manager.str(), type_manager.int())
    );
}

#[test]
fn test_map_heterogeneous_keys_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ \"a\": 1, 2: 3 }", &type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn test_map_heterogeneous_values_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ \"a\": 1, \"b\": true }", &type_manager, &bump);
    assert!(result.is_err());
}

// ============================================================================
// FormatStr Tests
// ============================================================================

#[test]
fn test_format_str_no_interpolations() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("f\"hello\"", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.str());
}

#[test]
fn test_format_str_with_interpolations() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("f\"x = {x}\" where { x = 42 }", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.str());
}

#[test]
fn test_format_str_function_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source(
        "f\"func = {f}\" where { f = (x) => x }",
        &type_manager,
        &bump,
    );
    assert!(result.is_err());
}

#[test]
fn test_otherwise_same_types() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("1 otherwise 2", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn test_otherwise_type_mismatch_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This should fail because Int and Str don't match
    let result = analyze_source("1 otherwise \"error\"", &type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn test_otherwise_with_array_indexing() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This represents: array[0] otherwise "default" where array = ["foo"]
    // For now just test compatible types work
    let result = analyze_source("\"foo\" otherwise \"bar\"", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.str());
}
