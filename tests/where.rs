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
    single_line_where,
    input: "0   where{    a=1  ,b=2}",
    formatted: Ok("0 where { a = 1, b = 2 }"),
    // Format and normalize weird spacing.
);

test_case!(
    multi_line_where_newline_before,
    input: indoc! {"
        0
        where{a=1 + 2,b=3*4}"},
    formatted: Ok(indoc! {"
        0
        where { a = 1 + 2, b = 3 * 4 }"}),
);

test_case!(
    multi_line_where_bindings_on_separate_lines,
    input: indoc! {"
        0 where {a=1,
                 b=2}"},
    formatted: Ok(indoc! {"
        0 where {
            a = 1,
            b = 2,
        }"}),
);

test_case!(
    delete_trailing_comma_single_line,
    input: "0 where {a=1,b=2,}",
    formatted: Ok("0 where { a = 1, b = 2 }"),
);

test_case!(
    multi_line_where_respects_newlines,
    input: indoc! {"
        0 where {
        a=1, b=2}"},
    formatted: Ok(indoc! {"
        0 where {
            a = 1, b = 2,
        }"}),
);

test_case!(
    where_with_comments,
    input: indoc! {"
        0 where {
            a = 1// first binding
            , b = 2   // second binding
            , c = 30 // third binding
        }"},
    formatted: Ok(indoc! {"
        0 where {
            a = 1,   // first binding
            b = 2,   // second binding
            c = 30,  // third binding
        }"}),
    // Comments in where bindings - should align vertically and add trailing comma
);
