use alloc::sync::Arc;

use super::*;
use crate::format;
use crate::{
    errors::{Error, ErrorKind},
    parser,
    types::{Type, manager::TypeManager},
};
use bumpalo::Bump;

// Helper to parse and analyze a source string
fn analyze_source<'types, 'arena>(
    source: &'arena str,
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
) -> Result<&'arena typed_expr::TypedExpr<'types, 'arena>, Error>
where
    'types: 'arena,
{
    let parsed = parser::parse(arena, source).map_err(|e| Error {
        kind: Arc::new(ErrorKind::Parse {
            src: source.to_string(),
            err_span: parser::Span::new(0, 0),
            help: Some(format!("Failed to parse source: {}", e)),
        }),
        context: vec![],
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
        assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.int()));
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
        assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.float()));
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
        assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.bool()));
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
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.int()));

    let result = analyze_source("-(3.14)", &type_manager, &bump);
    assert!(result.is_ok());
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.float()));

    // Also test with a variable to ensure it works on non-literals
    let result = analyze_source("-x where { x = 5 }", &type_manager, &bump);
    assert!(result.is_ok());
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.int()));
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
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.bool()));
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
    assert!(core::ptr::eq(
        int_result.unwrap().expr.0,
        type_manager.int()
    ));

    let float_result = analyze_source("3.14", &type_manager, &bump);
    assert!(float_result.is_ok());
    assert!(core::ptr::eq(
        float_result.unwrap().expr.0,
        type_manager.float()
    ));

    let bool_result = analyze_source("true", &type_manager, &bump);
    assert!(bool_result.is_ok());
    assert!(core::ptr::eq(
        bool_result.unwrap().expr.0,
        type_manager.bool()
    ));

    let str_result = analyze_source("\"hello\"", &type_manager, &bump);
    assert!(str_result.is_ok());
    assert!(core::ptr::eq(
        str_result.unwrap().expr.0,
        type_manager.str()
    ));
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

    assert!(core::ptr::eq(result.expr.0, expected_type));
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
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.int()));

    let result = analyze_source("\"hello\" as String", &type_manager, &bump);
    assert!(result.is_ok());
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.str()));

    let result = analyze_source("3.14 as Float", &type_manager, &bump);
    assert!(result.is_ok());
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.float()));
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
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.int()));
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
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.int()));
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
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.int()));
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
    assert!(core::ptr::eq(
        result.unwrap().expr.0,
        type_manager.array(type_manager.int())
    ));
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
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.int()));
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
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.int()));
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
    assert!(core::ptr::eq(
        result.unwrap().expr.0,
        type_manager.record(vec![])
    ));
}

#[test]
fn test_record_single_field() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ x = 42 }", &type_manager, &bump);
    assert!(result.is_ok());
    let result = result.unwrap();
    let expected = type_manager.record(vec![("x", type_manager.int())]);
    assert!(core::ptr::eq(result.expr.0, expected));
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
    assert!(core::ptr::eq(result.expr.0, expected));
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
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.int()));
}

#[test]
fn test_record_field_access_multiple_fields() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ x = 42, y = \"hello\" }.y", &type_manager, &bump);
    assert!(result.is_ok());
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.str()));
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
#[cfg_attr(
    not(feature = "experimental_maps"),
    ignore = "Maps gated behind 'experimental_maps' feature flag"
)]
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
#[cfg_attr(
    not(feature = "experimental_maps"),
    ignore = "Maps gated behind 'experimental_maps' feature flag"
)]
fn test_map_homogeneous_types() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ \"a\": 1, \"b\": 2 }", &type_manager, &bump);
    assert!(result.is_ok());
    assert!(core::ptr::eq(
        result.unwrap().expr.0,
        type_manager.map(type_manager.str(), type_manager.int())
    ));
}

#[test]
#[cfg_attr(
    not(feature = "experimental_maps"),
    ignore = "Maps gated behind 'experimental_maps' feature flag"
)]
fn test_map_heterogeneous_keys_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ \"a\": 1, 2: 3 }", &type_manager, &bump);
    let err = result.unwrap_err();
    // Should fail with type checking error (heterogeneous keys), not MapsNotYetImplemented
    assert!(matches!(
        err.kind.as_ref(),
        crate::errors::ErrorKind::TypeChecking { .. }
    ));
}

#[test]
#[cfg_attr(
    not(feature = "experimental_maps"),
    ignore = "Maps gated behind 'experimental_maps' feature flag"
)]
fn test_map_heterogeneous_values_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ \"a\": 1, \"b\": true }", &type_manager, &bump);
    let err = result.unwrap_err();
    // Should fail with type checking error (heterogeneous values), not MapsNotYetImplemented
    assert!(matches!(
        err.kind.as_ref(),
        crate::errors::ErrorKind::TypeChecking { .. }
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
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.str()));
}

#[test]
fn test_format_str_with_interpolations() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("f\"x = {x}\" where { x = 42 }", &type_manager, &bump);
    assert!(result.is_ok());
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.str()));
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
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.int()));
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
    assert!(core::ptr::eq(result.unwrap().expr.0, type_manager.str()));
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
        Err(Error { kind, .. }) => match kind.as_ref() {
            ErrorKind::TypeChecking { help, .. } => {
                assert!(help.as_ref().unwrap().contains("suffixes"));
            }
            _ => panic!("Expected TypeChecking error"),
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
        Err(Error { kind, .. }) => match kind.as_ref() {
            ErrorKind::TypeChecking { help, .. } => {
                assert!(help.as_ref().unwrap().contains("suffixes"));
            }
            _ => panic!("Expected TypeChecking error"),
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

    // The result should be a record with fields a: Int and b: Str
    let typed = result.unwrap();
    if let Type::Record(fields) = typed.expr.0 {
        assert_eq!(fields.len(), 2);
        // Fields are sorted: a, b
        assert_eq!(fields[0].0, "a");
        assert!(core::ptr::eq(fields[0].1, type_manager.int()));
        assert_eq!(fields[1].0, "b");
        assert!(core::ptr::eq(fields[1].1, type_manager.str()));
    } else {
        panic!("Expected Record type, got {:?}", typed.expr.0);
    }
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
    if let Type::Array(elem_ty) = typed.expr.0 {
        assert!(core::ptr::eq(*elem_ty, type_manager.int()));
    } else {
        panic!("Expected Array type, got {:?}", typed.expr.0);
    }
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

    // Verify types
    let typed = result.unwrap();
    if let Type::Record(fields) = typed.expr.0 {
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].0, "a");
        assert!(core::ptr::eq(fields[0].1, type_manager.int()));
        assert_eq!(fields[1].0, "b");
        assert!(core::ptr::eq(fields[1].1, type_manager.str()));
    } else {
        panic!("Expected Record type");
    }
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
            wrap_result = wrap(id(99))
        }
        where {
            id = (x) => x,
            wrap = (x) => [x]
        }
    "#;
    let result = analyze_source(source, &type_manager, &bump);

    assert!(
        result.is_ok(),
        "Sequential polymorphic bindings should typecheck: {:?}",
        result
    );

    let typed = result.unwrap();
    if let Type::Record(fields) = typed.expr.0 {
        assert_eq!(fields.len(), 3);
        assert!(core::ptr::eq(fields[0].1, type_manager.int()));
        assert!(core::ptr::eq(fields[1].1, type_manager.str()));
        // wrap_result should be Array[Int]
        if let Type::Array(elem_ty) = fields[2].1 {
            assert!(core::ptr::eq(*elem_ty, type_manager.int()));
        } else {
            panic!("Expected Array type for wrap_result");
        }
    } else {
        panic!("Expected Record type");
    }
}

#[test]
#[ignore = "implement support for passing polymorphic functions as arguments (higher-rank polymorphism)"]
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
    assert!(core::ptr::eq(typed.expr.0, type_manager.int()));
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
    if let Type::Array(elem_ty) = typed.expr.0 {
        assert!(core::ptr::eq(*elem_ty, type_manager.int()));
    } else {
        panic!("Expected Array type");
    }
}

#[test]
#[ignore = "nested where clauses with polymorphism need proper environment tracking"]
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
#[ignore = "requires higher-rank polymorphism to pass functions as arguments"]
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
    if let Type::Record(fields) = typed.expr.0 {
        assert_eq!(fields.len(), 2);
        assert!(core::ptr::eq(fields[0].1, type_manager.int()));
        assert!(core::ptr::eq(fields[1].1, type_manager.str()));
    } else {
        panic!("Expected Record type");
    }
}

#[test]
#[ignore = "type variables not being resolved correctly in nested polymorphic calls"]
fn test_polymorphic_compose() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Function composition with polymorphism
    // Type checks successfully but type variables aren't fully resolved
    let source = r#"
        {
            result1 = wrap(1),
            result2 = wrap("test")
        }
        where {
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
    if let Type::Record(fields) = typed.expr.0 {
        assert_eq!(fields.len(), 2);
        // result1 should be Array[Int]
        if let Type::Array(elem_ty) = fields[0].1 {
            assert!(core::ptr::eq(*elem_ty, type_manager.int()));
        } else {
            panic!("Expected Array[Int] for result1");
        }
        // result2 should be Array[Str]
        if let Type::Array(elem_ty) = fields[1].1 {
            assert!(core::ptr::eq(*elem_ty, type_manager.str()));
        } else {
            panic!("Expected Array[Str] for result2");
        }
    } else {
        panic!("Expected Record type");
    }
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

    // Should fail with type mismatch: Bool vs Int
    assert!(
        result.is_err(),
        "Closure capturing lambda parameter should not be polymorphic - Bool + Int should fail"
    );
}
