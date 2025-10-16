/*
 * REVIEWED & LOCKED
 *
 * This test file has been reviewed and its expectations locked in.
 * Any future changes to these test expectations must be explicitly
 * discussed and agreed upon before implementation.
 *
 * Last reviewed: October 15, 2025
 */

use melbi_core::parser::UnaryOp;

mod cases;

test_case!(
    otherwise_operator,
    input: "x otherwise 0",
    ast: Ok(Expr::Otherwise {
        primary: Expr::Ident("x"),
        fallback: Expr::Literal(Literal::Int {value: 0, suffix: None})
    }),
    formatted: Ok("x otherwise 0"),
);

test_case!(
    negation,
    input: "-5",
    ast: Ok(Expr::Literal(Literal::Int { value: -5, suffix: None })),
    formatted: Ok("-5"),
);

test_case!(
    negation_with_spaces,
    input: "  -  x  ",
    ast: Ok(Expr::Unary {
        op: UnaryOp::Neg,
        expr: Expr::Ident("x")
    }),
    formatted: Ok("-x"),
    // Trim spaces around unary minus
);

test_case!(
    logical_not_with_spaces,
    input: "  not   true  ",
    formatted: Ok("not true"),
    // Trim spaces around logical not
);

test_case!(
    otherwise_with_complex_expressions,
    input: "x + y otherwise a * b",
    formatted: Ok("x + y otherwise a * b"),
    // Otherwise with complex expressions
);

test_case!(
    double_negation,
    input: "--5",
    formatted: Ok("--5"),
    // Double unary operators
);

test_case!(
    not_with_parentheses,
    input: "not(true)",
    formatted: Ok("not (true)"),
    // Parentheses after not
);

test_case!(
    operators_with_comments,
    input: "- 5 // negation",
    formatted: Ok("-5  // negation"),
    // Comments after unary operators
);
