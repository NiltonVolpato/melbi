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
    empty_array,
    input: "[]",
    formatted: Ok("[]"),
);

test_case!(
    single_line_array,
    input: "[1,2,3]",
    formatted: Ok("[1, 2, 3]"),
);

test_case!(
    multi_line_array,
    input: indoc! {"
        [1,
         2,3]"},
    formatted: Ok(indoc! {"
        [
            1,
            2,
            3,
        ]"}),
);

test_case!(
    delete_trailing_comma_single_line,
    input: "[1,2,3,]",
    formatted: Ok("[1, 2, 3]"),
);

test_case!(
    multi_line_array_respects_newlines,
    input: indoc! {"
        [
        1,2,3]"},
    formatted: Ok(indoc! {"
        [
            1, 2, 3,
        ]"}),
);

test_case!(
    nested_arrays,
    input: "[[1,2],[3,4]]",
    formatted: Ok("[[1, 2], [3, 4]]"),
);

test_case!(
    array_with_expressions,
    input: "[1+2, 3*4, 5-6]",
    formatted: Ok("[1 + 2, 3 * 4, 5 - 6]"),
);

test_case!(
    array_with_weird_spacing,
    input: "[  1  ,\t2,3   ]",
    formatted: Ok("[1, 2, 3]"),
    // Normalize weird whitespace and tabs
);

test_case!(
    array_with_weird_spacing_and_newlines,
    input: indoc! {"
        [  1  ,
        \t2,3   ]
    "},
    formatted: Ok(indoc!{"
        [
            1,
            2,
            3,
        ]"}),
    // Normalize weird whitespace and newline implies multi-line
);

test_case!(
    array_with_comments,
    input: indoc! {"
        [
            1,     // first
            2,// second
            3 // third
        ]"},
    formatted: Ok(indoc! {"
        [
            1,   // first
            2,   // second
            3,   // third
        ]"}),
    // Comments in arrays - should align vertically and add trailing comma
);

test_case!(
    array_mixed_expressions,
    input: "[1+2*3, (4-5)/6, 7^8]",
    formatted: Ok("[1 + 2 * 3, (4 - 5) / 6, 7 ^ 8]"),
    // Complex expressions with operator precedence
);

test_case!(
    array_with_where,
    input: "[1+2*3, (a-b)/c where {a = 4, b = 5, c = 6}, 7^8]",
    formatted: Ok("[1 + 2 * 3, (a - b) / c where { a = 4, b = 5, c = 6 }, 7 ^ 8]"),
    // Complex expressions with operator precedence
);

test_case!(
    array_with_multiline_where,
    input: indoc! {"
        [1+2*3, (a-b)/c where {
                            a = 4,
                            b = 5,
                            c = 6
                        }, 7^8]"},
    formatted: Ok(indoc! {"
        [
            1 + 2 * 3,
            (a - b) / c where {
                a = 4,
                b = 5,
                c = 6,
            },
            7 ^ 8
        ]"}),
    // Complex expressions with operator precedence
);

test_case!(
    array_empty_with_whitespace,
    input: "[\n\n]",
    formatted: Ok("[]"),
    // Empty array with newlines inside
);

test_case!(
    array_single_element_trailing_comma,
    input: "[42,]",
    formatted: Ok("[42]"),
    // Single element with trailing comma - should remove it
);
