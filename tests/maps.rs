mod cases;

use indoc::indoc;

test_case!(
    empty_map,
    input: "{}",
    formatted: Ok("{}"),
);

test_case!(
    single_line_map,
    input: "{a:1,b:2}",
    formatted: Ok("{ a : 1, b : 2 }"),
);

test_case!(
    multi_line_map,
    input: indoc! {"
        {a:1,
         b:2}"},
    formatted: Ok(indoc! {"
        {
            a : 1,
            b : 2,
        }"}),
);

test_case!(
    delete_trailing_comma_single_line,
    input: "{a:1,b:2,}",
    formatted: Ok("{ a : 1, b : 2 }"),
);

test_case!(
    multi_line_map_respects_newlines,
    input: indoc! {"
        {
        a:1, b:2}"},
    formatted: Ok(indoc! {"
        {
            a : 1, b : 2,
        }"}),
);

test_case!(
    map_with_complex_keys,
    input: "{1+2:3, 4:5*6}",
    formatted: Ok("{ 1 + 2 : 3, 4 : 5 * 6 }"),
);
