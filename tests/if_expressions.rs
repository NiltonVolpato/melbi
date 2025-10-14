mod cases;

use indoc::indoc;

test_case!(
    simple_if,
    input: "if true then 1 else 2",
    formatted: Ok("if true then 1 else 2"),
);

test_case!(
    if_with_newline_before_then,
    input: indoc! {"
        if true
        then 1 else 2"},
    formatted: Ok(indoc! {"
        if true
        then 1 else 2"}),
);

test_case!(
    if_with_newline_before_else,
    input: indoc! {"
        if true then 1
        else 2"},
    formatted: Ok(indoc! {"
        if true then 1
        else 2"}),
);

test_case!(
    if_all_on_separate_lines,
    input: indoc! {"
        if true
        then 1
        else 2"},
    formatted: Ok(indoc! {"
        if true
        then 1
        else 2"}),
);

test_case!(
    nested_if,
    input: "if a then if b then 1 else 2 else 3",
    formatted: Ok("if a then if b then 1 else 2 else 3"),
);
