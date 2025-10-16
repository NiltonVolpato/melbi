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
    simple_if,
    input: "if true then 1 else 2",
    formatted: "if true then 1 else 2",
);

test_case!(
    if_with_newline_before_then,
    input: indoc! {"
        if true
        then 1 else 2"},
    formatted: r#"
if true
then 1 else 2"#.trim_start(),
);

test_case!(
    if_with_newline_before_else,
    input: indoc! {"
        if true then 1
        else 2"},
    formatted: r#"
if true then 1
else 2"#.trim_start(),
);

test_case!(
    if_all_on_separate_lines,
    input: indoc! {"
        if true
        then 1
        else 2"},
    formatted: r#"
if true
then 1
else 2"#.trim_start(),
);

test_case!(
    nested_if,
    input: "if a then if b then 1 else 2 else 3",
    formatted: "if a then if b then 1 else 2 else 3",
);

test_case!(
    if_with_weird_spacing,
    input: "if   true   then   1   else   2",
    formatted: "if true then 1 else 2",
);
// Normalize weird spacing around keywords

test_case!(
    if_with_comments,
    input: indoc! {"
        if true then  // condition
            1        // then branch
        else         // else branch
            2"},
    formatted: r#"
if true
then  // condition
    1  // then branch
else  // else branch
    2"#.trim_start(),
);
// Preserve comments and indentation

test_case!(
    if_complex_condition,
    input: "if a and b or c then x + y else z * w",
    formatted: "if a and b or c then x + y else z * w",
);
// Complex boolean conditions

test_case!(
    if_multiline_condition,
    input: indoc! {"
        if a and
           b then 1 else 2"},
    formatted: r#"
if a and b then 1 else 2"#.trim_start(),
);
// Multi-line conditions

test_case!(
    if_multiline_branches,
    input: indoc! {"
        if true then
            x + y +
            z
        else
            a * b -
            c"},
    formatted: r#"
if true
then x + y + z
else a * b - c"#.trim_start(),
);
// Multi-line branches
