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
    input: "{x =  1   ,    y     =      2       }",
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
    input: "{x=1+2,y=3*4,}",
    formatted: Ok("{ x = 1 + 2, y = 3 * 4 }"),
);

test_case!(
    multi_line_record_respects_newlines,
    input: indoc! {"
        {
        x=1,y={z=3}}"},
    formatted: Ok(indoc! {"
        {
            x = 1, y = { z = 3 },
        }"}),
);

test_case!(
    record_with_comments,
    input: indoc! {"
        {
            x = 1, // first

            y = 10,// second
            z = 100     // third
        }"},
    formatted: Ok(indoc! {"
        {
            x = 1,    // first
            y = 10,   // second
            z = 100,  // third
        }"}),
    // Comments in records - should align vertically and add trailing comma
);
