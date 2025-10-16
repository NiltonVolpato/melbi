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

use indoc::indoc;

test_case!(
    arithmetic_addition,
    input: "1+2",
    formatted: "1 + 2",
);

test_case!(
    arithmetic_subtraction,
    input: "3-4",
    formatted: "3 - 4",
);

test_case!(
    arithmetic_multiplication,
    input: "5*6",
    formatted: "5 * 6",
);

test_case!(
    arithmetic_division,
    input: "7/8",
    formatted: "7 / 8",
);

test_case!(
    arithmetic_power,
    input: "2^3",
    formatted: "2 ^ 3",
);

test_case!(
    logical_and,
    input: "true and false",
    formatted: "true and false",
);

test_case!(
    logical_or,
    input: "true or false",
    formatted: "true or false",
);

test_case!(
    operator_precedence,
    input: "1+2*3",
    formatted: "1 + 2 * 3",
);

test_case!(
    operator_precedence_with_parens,
    input: "(1+2)*3",
    formatted: "(1 + 2) * 3",
);

test_case!(
    weird_spacing,
    input: "  1   +    2  ",
    formatted: "1 + 2",
);

test_case!(
    chained_operations,
    input: "1+2-3*4/5",
    formatted: "1 + 2 - 3 * 4 / 5",
);

test_case!(
    complex_expression,
    input: "2+3*4-5/2",
    formatted: "2 + 3 * 4 - 5 / 2",
);

test_case!(
    parenthesized_expression,
    input: "(2+3)*(4-1)",
    formatted: "(2 + 3) * (4 - 1)",
);

test_case!(
    binary_with_comments,
    input: "1 + 2// addition",
    formatted: "1 + 2  // addition",
    // 2 spaces before end-of-line comments (Google style)
);

test_case!(
    multiline_expression,
    input: indoc! {"
        1 +
        2 *
        3"},
    formatted: "1 + 2 * 3",
    // Write simple expressions on a single line
);
