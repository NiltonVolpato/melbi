mod cases;

use indoc::indoc;

test_case!(
    empty_record,
    input: "Record {}",
    formatted: Ok("Record{}"),
);

test_case!(
    empty_record_with_newlines,
    input: indoc!("
        Record

        {
        }"),
    formatted: Ok("Record{}"),
);

test_case!(
    single_line_record,
    input: "{x=1,y=2}",
    formatted: Ok("{ x = 1, y = 2 }"),
);

test_case!(
    multi_line_record,
    input: indoc! {"
        {x=1,
        y=2}"},
    formatted: Ok(indoc! {"
        {
            x = 1,
            y = 2,
        }"}),
);

test_case!(
    delete_trailing_comma_single_line,
    input: "{x=1,y=2,}",
    formatted: Ok("{ x = 1, y = 2 }"),
);

test_case!(
    multi_line_record_respects_newlines,
    input: indoc! {"
        {
        x=1,y=2}"},
    formatted: Ok(indoc! {"
        {
            x = 1, y = 2,
        }"}),
);
