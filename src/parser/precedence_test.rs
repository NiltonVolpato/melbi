use crate::ast::Expr;
use pest::Parser;

use super::{Rule, parser::parse_expr};

fn p(source: &str) -> Expr {
    let mut pairs = super::ExpressionParser::parse(Rule::main, source)
        .unwrap_or_else(|e| panic!("Parsing failed:\n{}", e));
    let pair = pairs.next().unwrap();
    parse_expr(pair)
}

#[test]
fn test_addition_vs_subtraction() {
    assert_eq!(p("a + b - c"), p("(a + b) - c"));
    assert_eq!(p("a - b + c"), p("(a - b) + c"));
    assert_eq!(
        p("a + b - c + d - e + f"),
        p("((((a + b) - c) + d) - e) + f")
    );
}

#[test]
fn test_multiplication_vs_division() {
    assert_eq!(p("a * b / c"), p("(a * b) / c"));
    assert_eq!(p("a / b * c"), p("(a / b) * c"));
    assert_eq!(
        p("a * b / c * d / e * f"),
        p("((((a * b) / c) * d) / e) * f")
    );
}

#[test]
fn test_addition_vs_multiplication() {
    assert_eq!(p("a + b * c"), p("a + (b * c)"));
    assert_eq!(p("a * b + c"), p("(a * b) + c"));
}

#[test]
fn test_and_vs_or() {
    assert_eq!(p("true and false or true"), p("(true and false) or true"));
    assert_eq!(p("true or false and true"), p("true or (false and true)"));
}

#[test]
fn test_unary_vs_binary() {
    assert_eq!(p("--a"), p("-(-a)"));
    assert_eq!(p("-a + b"), p("(-a) + b"));

    // TODO: "a + -b" raises a parsing error currently, but it should be equivalent to "a + (-b)".
    // assert_eq!(p("a + -b)"), p("a + (-b)"));
}

#[test]
fn test_exponentiation() {
    assert_eq!(p("a ^ b ^ c"), p("a ^ (b ^ c)"));
    assert_eq!(p("a ^ b ^ c ^ d"), p("a ^ (b ^ (c ^ d))"));
}

#[test]
fn test_exponentiation_vs_multiplication() {
    assert_eq!(p("a * b ^ c"), p("a * (b ^ c)"));
    assert_eq!(p("(a * b) ^ c"), p("(a * b) ^ c"));
}

#[test]
fn test_exponentiation_vs_negation() {
    assert_eq!(p("-a ^ b"), p("- (a  ^ b)"));
    assert_eq!(p("a ^ -b"), p("a ^ ( -b )"));
}

#[test]
fn test_if_vs_binary() {
    assert_eq!(p("if a then b + c else d"), p("if a then (b + c) else d"));
    assert_eq!(p("if a then b else c + d"), p("if a then b else (c + d)"));
}

#[test]
fn test_lambda_vs_everything() {
    assert_eq!(p("-(a) => b"), p("-((a) => b)")); // This couldn't be any other way. And it makes no sense semantically.
    assert_eq!(p("(a) => a or b"), p("(a) => (a or b)"));
    assert_eq!(p("(a) => a + b"), p("(a) => (a + b)"));
    assert_eq!(p("(a) => a()"), p("(a) => (a())"));
    assert_eq!(p("(a) => a[0]"), p("(a) => (a[0])"));
    assert_eq!(p("(a) => a.field"), p("(a) => (a.field)"));
    assert_eq!(p("(a) => a as String"), p("(a) => (a as String)"));
    assert_eq!(
        p("(a) => a[0] otherwise ''"),
        p("(a) => (a[0] otherwise '')")
    );
    assert_eq!(p("(a) => a otherwise ''"), p("(a) => (a otherwise '')"));
    assert_eq!(
        p("(a) => a where { x = 1 }"),
        p("(a) => (a where { x = 1 })")
    );
    assert_eq!(
        p("(a) => a where { x = 1 } otherwise ''"),
        p("(a) => (a where { x = 1 }) otherwise ''")
    );
}

#[test]
fn test_where_vs_prefix_operations() {
    assert_eq!(p("-a where { x = 1 }"), p("(-a) where { x = 1 }"));
    assert_eq!(p("not a where { x = 1 }"), p("(not a) where { x = 1 }"));
}

#[test]
fn test_where_vs_binary() {
    assert_eq!(p("a + b where { x = 1 }"), p("(a + b) where { x = 1 }"));
    assert_eq!(p("a where { x = 1 } + b"), p("(a where { x = 1 }) + b"));
}

#[test]
fn test_otherwise_vs_binary() {
    assert_eq!(p("a + b otherwise c"), p("(a + b) otherwise c"));
    assert_eq!(p("a otherwise b + c"), p("a otherwise (b + c)"));
}

#[test]
fn test_cast_vs_binary() {
    assert_eq!(p("a + b as String"), p("a + (b as String)"));
    assert_eq!(p("a as String + b"), p("(a as String) + b"));
}

#[test]
fn test_cast_vs_field_accessor() {
    assert_eq!(p("a.b as String"), p("( a.b ) as String"));
    assert_eq!(p("a as String . b"), p("(a as String).b"));
}

#[test]
fn test_grouped_vs_binary() {
    assert_eq!(p("a + (b * c)"), p("a + (b * c)"));
    assert_eq!(p("(a + b) * c"), p("(a + b) * c"));
}

#[test]
fn test_record_vs_binary() {
    assert_eq!(p("a + { x = 1 }"), p("a + ({ x = 1 })"));
    assert_eq!(p("{ x = 1 } + a"), p("({ x = 1 }) + a"));
}

#[test]
fn test_map_vs_binary() {
    assert_eq!(p("a + { x: 1, y: 2 }"), p("a + ({ x: 1, y: 2 })"));
    assert_eq!(p("{ x: 1, y: 2 } + a"), p("({ x: 1, y: 2 }) + a"));
}

#[test]
fn test_array_vs_binary() {
    assert_eq!(p("a + [1, 2, 3]"), p("a + ([1, 2, 3])"));
    assert_eq!(p("[1, 2, 3] + a"), p("([1, 2, 3]) + a"));
}

#[test]
fn test_attr_access_vs_binary() {
    assert_eq!(p("a + obj.field"), p("a + (obj.field)"));
    assert_eq!(p("obj.field + a"), p("(obj.field) + a"));
}

#[test]
fn test_index_access_vs_binary() {
    assert_eq!(p("a + arr[0]"), p("a + (arr[0])"));
    assert_eq!(p("arr[0] + a"), p("(arr[0]) + a"));
}

#[test]
fn test_function_call_vs_binary() {
    assert_eq!(p("a + foo(1, 2)"), p("a + (foo(1, 2))"));
    assert_eq!(p("foo(1, 2) + a"), p("(foo(1, 2)) + a"));
}

#[test]
fn test_otherwise() {
    assert_eq!(
        p("a otherwise b otherwise c otherwise d"),
        p("a otherwise (b otherwise (c otherwise d))")
    );
}

#[test]
fn test_otherwise_vs_if() {
    assert_eq!(
        p("if a then b else c otherwise d"),
        p("(if a then b else c) otherwise d")
    );
}

#[test]
fn test_otherwise_vs_where() {
    assert_eq!(
        p("a where { x = 1 } otherwise b"),
        p("(a where { x = 1 }) otherwise b")
    );
    assert_eq!(
        p("a otherwise b where { x = 1 }"),
        p("(a otherwise b) where { x = 1 }")
    );
}

#[test]
fn test_if_vs_where() {
    assert_eq!(
        p("if a then b where { x = 1 } else c"),
        p("if a then (b where { x = 1 }) else c")
    );
    assert_eq!(
        p("if a then b else c where { x = 1 }"),
        p("(if a then b else c) where { x = 1 }")
    );
}

#[test]
fn test_otherwise_vs_cast() {
    assert_eq!(p("a as b otherwise c"), p("(a as b) otherwise c"));
    assert_eq!(p("a otherwise b as c"), p("a otherwise (b as c)"));
}

#[test]
fn test_if_vs_cast() {
    assert_eq!(p("if a then b as c else d"), p("if a then (b as c) else d"));
    assert_eq!(p("if a then b else c as d"), p("if a then b else (c as d)"));
}

#[test]
fn test_where_vs_cast() {
    assert_eq!(p("a as b where { x = 1 }"), p("(a as b) where { x = 1 }"));
    assert_eq!(p("a where { x = 1 } as b"), p("(a where { x = 1 }) as b"));
}

#[test]
fn test_otherwise_vs_grouped() {
    assert_eq!(p("a + (b otherwise c)"), p("a + (b otherwise c)"));
    assert_eq!(p("(a + b) otherwise c"), p("(a + b) otherwise c"));
}

#[test]
fn test_otherwise_vs_division_and_addition() {
    assert_eq!(p("a / b otherwise b + c"), p("(a / b) otherwise (b + c)"));
}

#[test]
fn test_otherwise_vs_and_or() {
    assert_eq!(
        p("a and b otherwise c or d"),
        p("(a and b) otherwise (c or d)")
    );
    assert_eq!(
        p("a otherwise b and c or d"),
        p("a otherwise ((b and c) or d)")
    );
}

#[test]
fn test_complex_nested_expression() {
    assert_eq!(
        p("if a then 0 else b + c where { x = 1 } otherwise d and e or f"),
        p("(if a then 0 else (b + c) where { x = 1 }) otherwise ((d and e) or f)")
    );
}

#[test]
fn test_excessive_parentheses() {
    assert_eq!(p("(((a + b)))"), p("a + b"));
}

#[test]
fn test_exponentiation_associativity() {
    assert_eq!(p("a ^ b ^ c"), p("a ^ (b ^ c)"));
}

#[test]
fn test_function_call_with_complex_arguments() {
    assert_eq!(p("foo(a + b, c * d)"), p("foo((a + b), (c * d))"));
}

#[test]
fn test_chained_constructs() {
    assert_eq!(
        p("if a then b where { x = 1 } otherwise c else d"),
        p("if a then (b where { x = 1 } otherwise c) else d")
    );

    assert_eq!(
        p("if a then b else c where { x = 1 } otherwise d"),
        p("((if a then b else c) where { x = 1 }) otherwise d")
    );
}

#[test]
fn test_not_vs_and_or() {
    assert_eq!(p("not a and b"), p("(not a) and b"));
    assert_eq!(p("a or not b"), p("a or (not b)"));
}

#[test]
fn test_deeply_nested_expressions() {
    assert_eq!(p("a + (b * (c - (d / e)))"), p("a + (b * (c - (d / e)))"));
}
