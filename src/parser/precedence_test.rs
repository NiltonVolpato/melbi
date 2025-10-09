use crate::parser::ast::Expr;
use bumpalo::Bump;

use super::parser::parse;

// Helper function to parse an expression and return the AST.
//
// We test precedence by comparing whether two expressions parenthesized in
// different ways yield the same AST.
fn ast<'a>(arena: &'a Bump, source: &'a str) -> &'a Expr<'a> {
    let parsed = parse(arena, source)
        .unwrap_or_else(|e| panic!("Expression parsing failed: {}\n{}", source, e));
    parsed.expr
}

#[test]
fn test_addition_vs_subtraction() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + b - c"), ast(&arena, "(a + b) - c"));
    assert_eq!(ast(&arena, "a - b + c"), ast(&arena, "(a - b) + c"));
    assert_eq!(
        ast(&arena, "a + b - c + d - e + f"),
        ast(&arena, "((((a + b) - c) + d) - e) + f")
    );
}

#[test]
fn test_multiplication_vs_division() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a * b / c"), ast(&arena, "(a * b) / c"));
    assert_eq!(ast(&arena, "a / b * c"), ast(&arena, "(a / b) * c"));
    assert_eq!(
        ast(&arena, "a * b / c * d / e * f"),
        ast(&arena, "((((a * b) / c) * d) / e) * f")
    );
}

#[test]
fn test_addition_vs_multiplication() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + b * c"), ast(&arena, "a + (b * c)"));
    assert_eq!(ast(&arena, "a * b + c"), ast(&arena, "(a * b) + c"));
}

#[test]
fn test_and_vs_or() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "true and false or true"),
        ast(&arena, "(true and false) or true")
    );
    assert_eq!(
        ast(&arena, "true or false and true"),
        ast(&arena, "true or (false and true)")
    );
}

#[test]
fn test_unary_vs_binary() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "--a"), ast(&arena, "-(-a)"));
    assert_eq!(ast(&arena, "-a + b"), ast(&arena, "(-a) + b"));

    // TODO: "a + -b" raises a parsing error currently, but it should be equivalent to "a + (-b)".
    // assert_eq!(ast(&arena, "a + -b"), ast(&arena, "a + (-b)"));
}

#[test]
fn test_exponentiation() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a ^ b ^ c"), ast(&arena, "a ^ (b ^ c)"));
    assert_eq!(
        ast(&arena, "a ^ b ^ c ^ d"),
        ast(&arena, "a ^ (b ^ (c ^ d))")
    );
}

#[test]
fn test_exponentiation_vs_multiplication() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a * b ^ c"), ast(&arena, "a * (b ^ c)"));
    assert_eq!(ast(&arena, "(a * b) ^ c"), ast(&arena, "(a * b) ^ c"));
}

#[test]
fn test_exponentiation_vs_negation() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "-a ^ b"), ast(&arena, "- (a  ^ b)"));
    assert_eq!(ast(&arena, "a ^ -b"), ast(&arena, "a ^ ( -b )"));
}

#[test]
fn test_if_vs_binary() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "if a then b + c else d"),
        ast(&arena, "if a then (b + c) else d")
    );
    assert_eq!(
        ast(&arena, "if a then b else c + d"),
        ast(&arena, "if a then b else (c + d)")
    );
}

#[test]
fn test_lambda_vs_everything() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "-(a) => b"), ast(&arena, "-((a) => b)")); // This couldn't be any other way. And it makes no sense semantically.
    assert_eq!(ast(&arena, "(a) => a or b"), ast(&arena, "(a) => (a or b)"));
    assert_eq!(ast(&arena, "(a) => a + b"), ast(&arena, "(a) => (a + b)"));
    assert_eq!(ast(&arena, "(a) => a()"), ast(&arena, "(a) => (a())"));
    assert_eq!(ast(&arena, "(a) => a[0]"), ast(&arena, "(a) => (a[0])"));
    assert_eq!(
        ast(&arena, "(a) => a.field"),
        ast(&arena, "(a) => (a.field)")
    );
    assert_eq!(
        ast(&arena, "(a) => a as String"),
        ast(&arena, "(a) => (a as String)")
    );
    assert_eq!(
        ast(&arena, "(a) => a[0] otherwise ''"),
        ast(&arena, "(a) => (a[0] otherwise '')")
    );
    assert_eq!(
        ast(&arena, "(a) => a otherwise ''"),
        ast(&arena, "(a) => (a otherwise '')")
    );
    assert_eq!(
        ast(&arena, "(a) => a where { x = 1 }"),
        ast(&arena, "(a) => (a where { x = 1 })")
    );
    assert_eq!(
        ast(&arena, "(a) => a where { x = 1 } otherwise ''"),
        ast(&arena, "(a) => (a where { x = 1 }) otherwise ''")
    );
}

#[test]
fn test_where_vs_prefix_operations() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "-a where { x = 1 }"),
        ast(&arena, "(-a) where { x = 1 }")
    );
    assert_eq!(
        ast(&arena, "not a where { x = 1 }"),
        ast(&arena, "(not a) where { x = 1 }")
    );
}

#[test]
fn test_where_vs_binary() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a + b where { x = 1 }"),
        ast(&arena, "(a + b) where { x = 1 }")
    );
    assert_eq!(
        ast(&arena, "a where { x = 1 } + b"),
        ast(&arena, "(a where { x = 1 }) + b")
    );
}

#[test]
fn test_otherwise_vs_binary() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a + b otherwise c"),
        ast(&arena, "(a + b) otherwise c")
    );
    assert_eq!(
        ast(&arena, "a otherwise b + c"),
        ast(&arena, "a otherwise (b + c)")
    );
}

#[test]
fn test_cast_vs_binary() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a + b as String"),
        ast(&arena, "a + (b as String)")
    );
    assert_eq!(
        ast(&arena, "a as String + b"),
        ast(&arena, "(a as String) + b")
    );
}

#[test]
fn test_cast_vs_field_accessor() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a.b as String"),
        ast(&arena, "( a.b ) as String")
    );
    assert_eq!(
        ast(&arena, "a as String . b"),
        ast(&arena, "(a as String).b")
    );
}

#[test]
fn test_grouped_vs_binary() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + (b * c)"), ast(&arena, "a + (b * c)"));
    assert_eq!(ast(&arena, "(a + b) * c"), ast(&arena, "(a + b) * c"));
}

#[test]
fn test_record_vs_binary() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + { x = 1 }"), ast(&arena, "a + ({ x = 1 })"));
    assert_eq!(ast(&arena, "{ x = 1 } + a"), ast(&arena, "({ x = 1 }) + a"));
}

#[test]
fn test_map_vs_binary() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a + { x: 1, y: 2 }"),
        ast(&arena, "a + ({ x: 1, y: 2 })")
    );
    assert_eq!(
        ast(&arena, "{ x: 1, y: 2 } + a"),
        ast(&arena, "({ x: 1, y: 2 }) + a")
    );
}

#[test]
fn test_array_vs_binary() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + [1, 2, 3]"), ast(&arena, "a + ([1, 2, 3])"));
    assert_eq!(ast(&arena, "[1, 2, 3] + a"), ast(&arena, "([1, 2, 3]) + a"));
}

#[test]
fn test_attr_access_vs_binary() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + obj.field"), ast(&arena, "a + (obj.field)"));
    assert_eq!(ast(&arena, "obj.field + a"), ast(&arena, "(obj.field) + a"));
}

#[test]
fn test_index_access_vs_binary() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + arr[0]"), ast(&arena, "a + (arr[0])"));
    assert_eq!(ast(&arena, "arr[0] + a"), ast(&arena, "(arr[0]) + a"));
}

#[test]
fn test_function_call_vs_binary() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + foo(1, 2)"), ast(&arena, "a + (foo(1, 2))"));
    assert_eq!(ast(&arena, "foo(1, 2) + a"), ast(&arena, "(foo(1, 2)) + a"));
}

#[test]
fn test_otherwise() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a otherwise b otherwise c otherwise d"),
        ast(&arena, "a otherwise (b otherwise (c otherwise d))")
    );
}

#[test]
fn test_otherwise_vs_if() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "if a then b else c otherwise d"),
        ast(&arena, "(if a then b else c) otherwise d")
    );
}

#[test]
fn test_otherwise_vs_where() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a where { x = 1 } otherwise b"),
        ast(&arena, "(a where { x = 1 }) otherwise b")
    );
    assert_eq!(
        ast(&arena, "a otherwise b where { x = 1 }"),
        ast(&arena, "(a otherwise b) where { x = 1 }")
    );
}

#[test]
fn test_if_vs_where() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "if a then b where { x = 1 } else c"),
        ast(&arena, "if a then (b where { x = 1 }) else c")
    );
    assert_eq!(
        ast(&arena, "if a then b else c where { x = 1 }"),
        ast(&arena, "(if a then b else c) where { x = 1 }")
    );
}

#[test]
fn test_otherwise_vs_cast() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a as b otherwise c"),
        ast(&arena, "(a as b) otherwise c")
    );
    assert_eq!(
        ast(&arena, "a otherwise b as c"),
        ast(&arena, "a otherwise (b as c)")
    );
}

#[test]
fn test_if_vs_cast() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "if a then b as c else d"),
        ast(&arena, "if a then (b as c) else d")
    );
    assert_eq!(
        ast(&arena, "if a then b else c as d"),
        ast(&arena, "if a then b else (c as d)")
    );
}

#[test]
fn test_where_vs_cast() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a as b where { x = 1 }"),
        ast(&arena, "(a as b) where { x = 1 }")
    );
    assert_eq!(
        ast(&arena, "a where { x = 1 } as b"),
        ast(&arena, "(a where { x = 1 }) as b")
    );
}

#[test]
fn test_otherwise_vs_grouped() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a + (b otherwise c)"),
        ast(&arena, "a + (b otherwise c)")
    );
    assert_eq!(
        ast(&arena, "(a + b) otherwise c"),
        ast(&arena, "(a + b) otherwise c")
    );
}

#[test]
fn test_otherwise_vs_division_and_addition() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a / b otherwise b + c"),
        ast(&arena, "(a / b) otherwise (b + c)")
    );
}

#[test]
fn test_otherwise_vs_and_or() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a and b otherwise c or d"),
        ast(&arena, "(a and b) otherwise (c or d)")
    );
    assert_eq!(
        ast(&arena, "a otherwise b and c or d"),
        ast(&arena, "a otherwise ((b and c) or d)")
    );
}

#[test]
fn test_complex_nested_expression() {
    let arena = Bump::new();
    assert_eq!(
        ast(
            &arena,
            "if a then 0 else b + c where { x = 1 } otherwise d and e or f"
        ),
        ast(
            &arena,
            "(if a then 0 else (b + c) where { x = 1 }) otherwise ((d and e) or f)"
        )
    );
}

#[test]
fn test_excessive_parentheses() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "(((a + b)))"), ast(&arena, "a + b"));
}

#[test]
fn test_exponentiation_associativity() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a ^ b ^ c"), ast(&arena, "a ^ (b ^ c)"));
}

#[test]
fn test_function_call_with_complex_arguments() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "foo(a + b, c * d)"),
        ast(&arena, "foo((a + b), (c * d))")
    );
}

#[test]
fn test_chained_constructs() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "if a then b where { x = 1 } otherwise c else d"),
        ast(&arena, "if a then (b where { x = 1 } otherwise c) else d")
    );

    assert_eq!(
        ast(&arena, "if a then b else c where { x = 1 } otherwise d"),
        ast(&arena, "((if a then b else c) where { x = 1 }) otherwise d")
    );
}

#[test]
fn test_not_vs_and_or() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "not a and b"), ast(&arena, "(not a) and b"));
    assert_eq!(ast(&arena, "a or not b"), ast(&arena, "a or (not b)"));
}

#[test]
fn test_deeply_nested_expressions() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a + (b * (c - (d / e)))"),
        ast(&arena, "a + (b * (c - (d / e)))")
    );
}
