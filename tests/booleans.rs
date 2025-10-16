/*
 * REVIEWED & LOCKED
 *
 * This test file has been reviewed and its expectations locked in.
 * Any future changes to these test expectations must be explicitly
 * discussed and agreed upon before implementation.
 *
 * Last reviewed: October 15, 2025
 */

mod cases;

test_case!(
    true_literal,
    input: "true",
    formatted: "true",
);

test_case!(
    false_literal,
    input: "false",
    formatted: "false",
);

test_case!(
    boolean_with_spaces,
    input: "  true  ",
    formatted: "true",
);

test_case!(
    boolean_mixed_case,
    input: "True", // Not really a boolean, just an identifier
    ast: &Expr::Ident("True"),
    formatted: "True",
);

test_case!(
    boolean_uppercase,
    input: "FALSE", // Not really a boolean, just an identifier
    ast: &Expr::Ident("FALSE"),
    formatted: "FALSE",
);

test_case!(
    boolean_in_expression,
    input: "true   and    false",
    // Boolean operators - should this be formatted?
    formatted: "true and false",
);

test_case!(
    boolean_with_comment,
    input: "true // this is true",
    // Comments after booleans
    formatted: "true  // this is true",
);
