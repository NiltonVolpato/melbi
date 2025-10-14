mod cases;

use indoc::indoc;

test_case!(
    single_line_where,
    input: "0 where{a=1,b=2}",
    formatted: Ok("0 where { a = 1, b = 2 }"),
);

test_case!(
    multi_line_where_newline_before,
    input: indoc! {"
        0
        where{a=1,b=2}"},
    formatted: Ok(indoc! {"
        0
        where { a = 1, b = 2 }"}),
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
