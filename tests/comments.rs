mod cases;

use indoc::indoc;

// TODO: Comment formatting has many edge cases to handle
// Comments use `//` to end of line

test_case!(
    simple_comment,
    input: "// hello world",
    formatted: Ok("// hello world"),
);

test_case!(
    comment_after_expression,
    input: "42  // the answer",
    formatted: Ok("42 // the answer"),
    // Should preserve spacing before comment or normalize it?
);

test_case!(
    comment_in_record,
    input: indoc! {"
        {
            a = 1,  // first
            b = 2  // second
        }"},
    formatted: Ok(indoc! {"
        {
            a = 1, // first
            b = 2, // second
        }"}),
    // Comment attached to binding, trailing comma added
);

test_case!(
    comment_attached_to_comma,
    input: indoc! {"
        {
            a = 1,
            b = 7  // my favorite number
            ,
        }"},
    formatted: Ok(indoc! {"
        {
            a = 1,
            b = 7, // my favorite number
        }"}),
    // Tricky: comment between value and comma - should move with the comma?
);

test_case!(
    standalone_comment_in_block,
    input: indoc! {"
        {
            // This is a standalone comment
            a = 1,
            b = 2
        }"},
    formatted: Ok(indoc! {"
        {
            // This is a standalone comment
            a = 1,
            b = 2,
        }"}),
);

test_case!(
    multiple_comments,
    input: indoc! {"
        // Header comment
        // More header
        42  // inline comment
        "},
    formatted: Ok(indoc! {"
        // Header comment
        // More header
        42 // inline comment
        "}),
);

test_case!(
    comment_in_where,
    input: indoc! {"
        result where {
            // Calculate delta
            delta = b^2 - 4*a*c,
            // First root
            r0 = (-b + delta^0.5) / (2*a)  // positive discriminant
        }"},
    formatted: Ok(indoc! {"
        result where {
            // Calculate delta
            delta = b ^ 2 - 4 * a * c,
            // First root
            r0 = (- b + delta ^ 0.5) / (2 * a), // positive discriminant
        }"}),
);

test_case!(
    comment_after_operator,
    input: indoc! {"
        a + // what comes next?
        b"},
    formatted: Ok(indoc! {"
        a + // what comes next?
        b"}),
    // Comment after operator - keep on same line or move?
);

test_case!(
    comment_in_format_string,
    input: indoc! {r#"
        f"Hello {
            // arbitrary expressions accepted
            "Copilot"
        } from Melbi 🖖"
    "#},
    formatted: Ok(indoc! {r#"
        f"Hello {
            // arbitrary expressions accepted
            "Copilot"
        } from Melbi 🖖"
    "#}),
    // Comments inside format string interpolations
);
