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
    formatted: Ok("true"),
);

test_case!(
    false_literal,
    input: "false",
    formatted: Ok("false"),
);

test_case!(
    boolean_with_spaces,
    input: "  true  ",
    formatted: Ok("true"),
);

test_case!(
    boolean_mixed_case,
    input: "True",
    formatted: Err(_),
    // Normalize to lowercase
);

test_case!(
    boolean_uppercase,
    input: "FALSE",
    formatted: Err(_),
    // Normalize to lowercase
);

test_case!(
    boolean_in_expression,
    input: "true   and    false",
    formatted: Ok("true and false"),
    // Boolean operators - should this be formatted?
);

test_case!(
    boolean_with_comment,
    input: "true // this is true",
    formatted: Ok("true  // this is true"),
    // Comments after booleans
);
