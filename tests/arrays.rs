mod cases;

use indoc::indoc;

test_case!(
    empty_array,
    input: "[]",
    formatted: Ok("[ ]"),
);

test_case!(
    single_line_array,
    input: "[1,2,3]",
    formatted: Ok("[ 1, 2, 3 ]"),
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
    formatted: Ok("[ 1, 2, 3 ]"),
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
    formatted: Ok("[ [ 1, 2 ], [ 3, 4 ] ]"),
);

test_case!(
    array_with_expressions,
    input: "[1+2, 3*4, 5-6]",
    formatted: Ok("[ 1 + 2, 3 * 4, 5 - 6 ]"),
);
