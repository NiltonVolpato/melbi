use super::*;
use crate::format;
use crate::{
    analyzer::error::{TypeError, TypeErrorKind},
    parser,
    types::manager::TypeManager,
};
use bumpalo::Bump;

// Helper to parse and analyze a source string
fn analyze_source<'types, 'arena>(
    source: &'arena str,
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
) -> Result<&'arena typed_expr::TypedExpr<'types, 'arena>, TypeError>
where
    'types: 'arena,
{
    let parsed = parser::parse(arena, source).map_err(|e| {
        TypeError::new(TypeErrorKind::Other {
            message: format!("Failed to parse source: {}", e),
            span: parser::Span::new(0, 0),
        })
    })?;
    analyze(type_manager, arena, &parsed, &[], &[])
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

// ============================================================================
// Type Resolution Tests (Type variables in generic contexts)
// ============================================================================

#[test]
fn test_index_in_generic_lambda() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Lambda parameter 'arr' should be constrained to Indexable type class
    // With type classes: ((arr) => arr[0]) :: Indexable a => a -> element_type
    // When called with [1,2,3], unified to Array<Int> -> Int
    let source = "((arr) => arr[0])([1, 2, 3])";
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Index on generic lambda parameter should work with type classes: {:?}",
        result
    );
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn test_numeric_constraint_violation_with_source() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This should fail: trying to add a boolean in a generic lambda
    // The lambda parameter gets unified with Bool, then Numeric constraint fails
    let source = "((x, y) => x + y)(true, false)";
    let result = analyze_source(source, &type_manager, &bump);

    assert!(result.is_err(), "Should fail numeric constraint");

    let err = result.unwrap_err();
    // Verify error includes proper error kind
    match &err.kind {
        TypeErrorKind::ConstraintViolation { type_class, .. } => {
            assert!(
                type_class.contains("Numeric"),
                "Error should mention Numeric constraint"
            );
        }
        _ => panic!("Expected ConstraintViolation error, got: {:?}", err.kind),
    }
}

#[test]
fn test_nested_array_indexing_with_generic() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This tests that constraint checking handles partially-resolved types correctly
    // The inner array's element type might still be a type variable during constraint check
    // Array[Array[_t]] where _t is later resolved to Int
    // Both Array levels are Indexable regardless of what _t resolves to
    let source = "((arr) => arr[0][0])([[1, 2], [3, 4]])";
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Nested array indexing should work even with partially resolved generic types: {:?}",
        result
    );
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
#[ignore = "Requires row polymorphism - cannot infer 'any record with field x'"]
fn test_field_access_in_generic_lambda() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Lambda parameter 'r' should be constrained to "any record with field x"
    // With row polymorphism: ((r) => r.x) :: {x :: Int | r} -> Int
    // When called with {x: 42}, unified to Record{x: Int} -> Int
    let source = "((r) => r.x)({x: 42})";
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Field access on generic lambda parameter should work with row polymorphism: {:?}",
        result
    );
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
#[ignore = "Requires row polymorphism - nested case"]
fn test_nested_generic_lambda_field_access() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Higher-order function: pass a record processor function
    // With row polymorphism: f :: {x :: Int | r} -> Int, result :: Int
    // Nested generic lambda composition
    let source = "((f) => f({x = 1}))((r) => r.x)";
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Nested generic lambda with field access should work with row polymorphism: {:?}",
        result
    );
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
#[ignore = "Cast validation happens during lambda body analysis, before unification"]
fn test_cast_on_lambda_parameter() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Lambda parameter 'x' should allow cast with delayed validation
    // With constraint system: generate cast constraint, validate after unification
    // When called with 42, x is unified to Int, then Int->Float cast validated
    let source = "((x) => x as Float)(42)";
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Cast on generic lambda parameter should work with delayed validation: {:?}",
        result
    );
    assert_eq!(result.unwrap().expr.0, type_manager.float());
}

#[test]
fn test_index_in_where_bound_variable() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This should work - 'arr' in where clause gets proper type
    // But good regression test in case where-bound variables have similar issues
    let source = "arr[0] where { arr = if true then [1, 2, 3] else [4, 5, 6] }";
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Index on where-bound variable should work: {:?}",
        result
    );
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn test_field_access_in_where_bound_variable() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Similar to above - where-bound variable field access
    // Note: Records use '=' not ':' for field assignment
    let source = "r.x where { r = if true then {x = 1} else {x = 2} }";
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Field access on where-bound variable should work: {:?}",
        result
    );
    assert_eq!(result.unwrap().expr.0, type_manager.int());
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

#[test]
fn test_plus_one_lambda() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let result = analyze_source(
        "plus_one(9) where { plus_one = (a) => a + 1 }",
        &type_manager,
        &arena,
    );
    assert!(result.unwrap().expr.0 == type_manager.int());
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
    let result = result.unwrap();

    let expected_type = type_manager.record(vec![
        ("int", type_manager.int()),
        ("float", type_manager.float()),
        ("bool", type_manager.bool()),
        ("str", type_manager.str()),
        ("bytes", type_manager.bytes()),
    ]);

    assert_eq!(result.expr.0, expected_type);
}

// ============================================================================
// Cast
// ============================================================================

#[test]
fn test_cast_identity_allowed() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Identity casts are allowed (they're just no-ops)
    let result = analyze_source("42 as Int", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());

    let result = analyze_source("\"hello\" as String", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.str());

    let result = analyze_source("3.14 as Float", &type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.float());
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
    assert_eq!(result.unwrap().expr.0, type_manager.record(vec![]));
}

#[test]
fn test_record_single_field() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ x = 42 }", &type_manager, &bump);
    assert!(result.is_ok());
    let result = result.unwrap();
    let expected = type_manager.record(vec![("x", type_manager.int())]);
    assert_eq!(result.expr.0, expected);
}

#[test]
fn test_record_multiple_fields() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ x = 42, y = true, z = \"hello\" }", &type_manager, &bump);
    assert!(result.is_ok());
    let result = result.unwrap();
    let expected = type_manager.record(vec![
        ("x", type_manager.int()),
        ("y", type_manager.bool()),
        ("z", type_manager.str()),
    ]);
    assert_eq!(result.expr.0, expected);
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
    let err = result.unwrap_err();
    // Should fail with type mismatch error (heterogeneous keys)
    assert!(matches!(
        err.kind,
        TypeErrorKind::TypeMismatch { .. }
    ));
}

#[test]
fn test_map_heterogeneous_values_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ \"a\": 1, \"b\": true }", &type_manager, &bump);
    let err = result.unwrap_err();
    // Should fail with type mismatch error (heterogeneous values)
    assert!(matches!(
        err.kind,
        TypeErrorKind::TypeMismatch { .. }
    ));
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

// ============================================================================
// Cast Tests
// ============================================================================

#[test]
fn test_cast_invalid() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Int → Str is not supported (use format strings instead)
    let result = analyze_source("42 as Str", &type_manager, &bump);
    assert!(result.is_err());
    // Should fail because Int → Str is not a valid cast
}

// ============================================================================
// Literal Suffix Tests
// ============================================================================

#[test]
fn test_integer_suffix_not_supported() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("42`MB`", &type_manager, &bump);
    assert!(result.is_err());
    // Verify error message mentions suffixes
    match result {
        Err(TypeError { kind, .. }) => match kind {
            TypeErrorKind::Other { message, .. } => {
                assert!(message.contains("suffixes"));
            }
            _ => panic!("Expected Other error"),
        },
        Ok(_) => panic!("Expected suffix to fail"),
    }
}

// ============================================================================
// Span Tracking
// ============================================================================

#[test]
fn test_span_tracking_binary_expr() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "1 + 2";
    let result = analyze_source(source, &type_manager, &bump).unwrap();

    // Verify we have the annotation
    assert_eq!(result.ann.source, "1 + 2");

    // Check root expression span (the whole "1 + 2")
    let root_span = result.ann.span_of(result.expr);
    assert_eq!(root_span, Some(parser::Span::new(0, 5)));

    // Check sub-expressions have spans
    if let typed_expr::ExprInner::Binary { left, right, .. } = &result.expr.1 {
        // Left operand "1" should be at position 0..1
        let left_span = result.ann.span_of(left);
        assert_eq!(left_span, Some(parser::Span::new(0, 1)));

        // Right operand "2" should be at position 4..5
        let right_span = result.ann.span_of(right);
        assert_eq!(right_span, Some(parser::Span::new(4, 5)));
    } else {
        panic!("Expected Binary expression");
    }
}

#[test]
#[ignore = "Span tracking logic needs to be fixed in parser"]
fn test_span_tracking_nested_expr() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "(1 + 2) * 3";
    let result = analyze_source(source, &type_manager, &bump).unwrap();

    // Root multiplication should span from first operand to last (0..11)
    // Note: there is currently a bug in the span tracking logic.
    assert_eq!(
        result.ann.span_of(result.expr),
        Some(parser::Span::new(0, 11))
    );

    // Verify nested expressions also have spans
    if let typed_expr::ExprInner::Binary { left, right, .. } = &result.expr.1 {
        // Left addition "(1 + 2)" spans the content inside parens: 1..6
        let left_span = result.ann.span_of(left);
        assert_eq!(left_span, Some(parser::Span::new(1, 6)));

        // Right "3" should be 10..11
        let right_span = result.ann.span_of(right);
        assert_eq!(right_span, Some(parser::Span::new(10, 11)));
    } else {
        panic!("Expected Binary expression");
    }
}

#[test]
fn test_span_tracking_boolean_expr() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "true and false";
    let result = analyze_source(source, &type_manager, &bump).unwrap();

    // Root should span the whole expression
    assert_eq!(
        result.ann.span_of(result.expr),
        Some(parser::Span::new(0, 14))
    );

    // Verify operands have spans
    if let typed_expr::ExprInner::Boolean { left, right, .. } = &result.expr.1 {
        // Left "true" should be 0..4
        let left_span = result.ann.span_of(left);
        assert_eq!(left_span, Some(parser::Span::new(0, 4)));

        // Right "false" should be 9..14
        let right_span = result.ann.span_of(right);
        assert_eq!(right_span, Some(parser::Span::new(9, 14)));
    } else {
        panic!("Expected Boolean expression");
    }
}

#[test]
fn test_float_suffix_not_supported() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("3.14`meters`", &type_manager, &bump);
    assert!(result.is_err());
    // Verify error message mentions suffixes
    match result {
        Err(TypeError { kind, .. }) => match kind {
            TypeErrorKind::Other { message, .. } => {
                assert!(message.contains("suffixes"));
            }
            _ => panic!("Expected Other error"),
        },
        Ok(_) => panic!("Expected suffix to fail"),
    }
}

// ============================================================================
// Polymorphic Let Bindings (Type Schemes)
// ============================================================================

#[test]
fn test_polymorphic_identity_function() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // The identity function should work with both Int and Str
    let source = "{ a = id(1), b = id(\"foo\") } where { id = (x) => x }";
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Polymorphic identity function should typecheck: {:?}",
        result
    );

    let typed = result.unwrap();
    assert_eq!(
        typed.expr.0,
        type_manager.record(vec![("a", type_manager.int()), ("b", type_manager.str())])
    );
}

#[test]
fn test_polymorphic_inline_lambda() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Inline lambda with polymorphic parameters
    let source = "((a, b) => [b, a])(10, 42)";
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Polymorphic inline lambda should typecheck: {:?}",
        result
    );

    // The result should be an Array[Int]
    let typed = result.unwrap();
    assert_eq!(typed.expr.0, type_manager.array(type_manager.int()));
}

#[test]
fn test_polymorphic_pair_function() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // pair function should work with different types (but arrays are homogeneous)
    let source = r#"
        {
            int_pair = pair(1, 2),
            str_pair = pair("a", "b"),
            bool_pair = pair(true, false)
        }
        where {
            pair = (x, y) => [x, y]
        }
    "#;
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Polymorphic pair function should typecheck: {:?}",
        result
    );
}

#[test]
fn test_polymorphic_const_function() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // const function: (x, y) => x (returns first argument, ignores second)
    let source = r#"
        {
            a = konst(42, "ignored"),
            b = konst("hello", true)
        }
        where {
            konst = (x, y) => x
        }
    "#;
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Polymorphic const function should typecheck: {:?}",
        result
    );

    let typed = result.unwrap();
    assert_eq!(
        typed.expr.0,
        type_manager.record(vec![("a", type_manager.int()), ("b", type_manager.str())])
    );
}

#[test]
fn test_sequential_polymorphic_bindings() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Sequential bindings where later bindings can use earlier polymorphic ones
    let source = r#"
        {
            id_result1 = id(42),
            id_result2 = id("hello"),
            wrap_result = wrap(id(99)),
        } where {
            id = (x) => x,
            wrap = (x) => [x],
        }
    "#;
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Sequential polymorphic bindings should typecheck: {:?}",
        result
    );

    let typed = result.unwrap();
    assert_eq!(
        typed.expr.0,
        type_manager.record(vec![
            ("id_result1", type_manager.int()),
            ("id_result2", type_manager.str()),
            ("wrap_result", type_manager.array(type_manager.int())),
        ])
    );
}

#[test]
fn test_higher_rank_polymorphism() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This requires passing a polymorphic function as an argument
    // Currently fails because we can't pass type schemes as values
    let source = r#"
        apply(id, 42) where {
            id = (x) => x,
            apply = (f, x) => f(x)
        }
    "#;
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Higher-rank polymorphism should typecheck: {:?}",
        result
    );

    let typed = result.unwrap();
    assert_eq!(typed.expr.0, type_manager.int());
}

#[test]
fn test_polymorphic_in_array_literal() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Array containing results of polymorphic function calls
    let source = r#"
        [id(1), id(2), id(3)] where { id = (x) => x }
    "#;
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Polymorphic function in array literal should typecheck: {:?}",
        result
    );

    let typed = result.unwrap();
    assert_eq!(typed.expr.0, type_manager.array(type_manager.int()));
}

#[test]
fn test_nested_where_with_polymorphism() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Nested where clauses, inner scope uses outer polymorphic binding
    let source = r#"
        result where {
            id = (x) => x,
            result = inner where {
                inner = { a = id(1), b = id("test") }
            }
        }
    "#;
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Nested where with polymorphism should typecheck: {:?}",
        result
    );
}

#[test]
fn test_polymorphic_function_type_error() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This should fail: trying to use id with inconsistent types in same context
    // where unification is required
    let source = r#"
        [id(1), id("mixed")] where { id = (x) => x }
    "#;
    let result = analyze_source(source, &type_manager, &bump);

    // Arrays are homogeneous, so id(1) fixes the array element type to Int,
    // then id("mixed") should fail because Str != Int
    assert!(
        result.is_err(),
        "Mixed types in homogeneous array should fail"
    );
}

#[test]
fn test_polymorphic_map_function() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Simple map-like function (not actual iteration, just demonstrates polymorphism)
    // Fails because apply_twice needs to accept a polymorphic function parameter
    let source = r#"
        {
            int_result = apply_twice((x) => x + 1, 5),
            str_result = apply_twice((s) => s, "hello")
        }
        where {
            apply_twice = (f, x) => f(f(x))
        }
    "#;
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Polymorphic apply_twice should typecheck: {:?}",
        result
    );

    let typed = result.unwrap();
    assert_eq!(
        typed.expr.0,
        type_manager.record(vec![
            ("int_result", type_manager.int()),
            ("str_result", type_manager.str()),
        ])
    );
}

#[test]
fn test_polymorphic_compose() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Function composition with polymorphism
    // Type checks successfully but type variables aren't fully resolved
    let source = r#"
        {
            result1 = wrap(1),
            result2 = wrap("test")
        } where {
            id = (x) => x,
            wrap = (x) => [id(x)]
        }
    "#;
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Polymorphic composition should typecheck: {:?}",
        result
    );

    let typed = result.unwrap();
    assert_eq!(
        typed.expr.0,
        type_manager.record(vec![
            ("result1", type_manager.array(type_manager.int())),
            ("result2", type_manager.array(type_manager.str())),
        ])
    );
}

#[test]
fn test_closure_capturing_lambda_param_should_not_be_polymorphic() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This tests that closures capturing lambda parameters are NOT polymorphic.
    // The parameter p has type Bool (from calling with true).
    // capture = () => p should have type () => Bool, NOT be polymorphic.
    // Therefore capture() + 1 should fail (can't add Bool + Int).
    let source = r#"
        ((p) => result where {
          capture = () => p,
          result = capture() + 1
        })(true)
    "#;
    let result = analyze_source(source, &type_manager, &bump);

    // Should fail with specific type mismatch: Bool vs Int
    match result {
        Err(err) => {
            let err_str = format!("{:?}", err);
            assert!(
                err_str.contains("TypeMismatch")
                    && (err_str.contains("Bool") && err_str.contains("Int")),
                "Expected TypeMismatch between Bool and Int, got: {:?}",
                err
            );
        }
        Ok(_) => panic!(
            "Expected type error: closure capturing lambda parameter should not be polymorphic"
        ),
    }
}
