// ============================================================================
// REVIEWED & LOCKED - Test expectations are set in stone
// Date: 2024-10-14
// All test expectations in this file have been reviewed and approved.
// DO NOT change expectations without explicit discussion.
// If tests fail, fix the formatter, not the tests.
// ============================================================================

mod cases;

use indoc::indoc;

// TODO: Fix string content being stripped by topiary formatter
// Bug: strings like "hello" get formatted as ""

test_case!(
    empty_string,
    input: r#""""#,
    formatted: Ok(r#""""#),
);

test_case!(
    simple_string,
    input: r#""hello""#,
    formatted: Ok(r#""hello""#),
    // Currently FAILS: outputs "" instead
);

test_case!(
    string_with_spaces,
    input: r#""hello world""#,
    formatted: Ok(r#""hello world""#),
);

test_case!(
    string_with_special_chars,
    input: r#""hello\nworld""#,
    formatted: Ok(r#""hello\nworld""#),
);

test_case!(
    empty_bytes,
    input: r#"b"""#,
    formatted: Ok(r#"b"""#),
);

test_case!(
    simple_bytes,
    input: r#"b"hello""#,
    formatted: Ok(r#"b"hello""#),
);

test_case!(
    bytes_with_escape,
    input: r#"b"\x48\x65\x6c\x6c\x6f""#,
    formatted: Ok(r#"b"\x48\x65\x6c\x6c\x6f""#),
);

test_case!(
    format_string_empty,
    input: r#"f"""#,
    formatted: Ok(r#"f"""#),
);

test_case!(
    format_string_simple,
    input: r#"f"Hello {name}""#,
    formatted: Ok(r#"f"Hello { name }""#),
);

test_case!(
    format_string_multiple_interpolations,
    input: r#"f"{x} + {y} = {x+y}""#,
    formatted: Ok(r#"f"{ x } + { y } = { x + y }""#),
    // Spacing around operators in interpolations AND around braces
);

test_case!(
    format_string_complex,
    input: r#"f"Result: {result where{x=1,y=2,result=x+y}}""#,
    formatted: Ok(r#"f"Result: { result where { x = 1, y = 2, result = x + y } }""#),
);

test_case!(
    format_string_multiline_expression,
    input: indoc! {r#"
        f"Hello {
            "Copilot"
        } from Melbi 🖖"
    "#},
    formatted: Ok(indoc! {r#"
        f"Hello {
            "Copilot"
        } from Melbi 🖖"
    "#}),
    // Should preserve user's newlines inside interpolations
);

test_case!(
    format_string_multiline_complex,
    input: indoc! {r#"
        f"Result: {
            result where {
                x = 1,
                y = 2,
                result = x + y
            }
        }"
    "#},
    formatted: Ok(indoc! {r#"
        f"Result: {
            result where {
                x = 1,
                y = 2,
                result = x + y,
            }
        }"
    "#}),
    // Format strings can contain arbitrarily complex multi-line expressions
);

test_case!(
    format_string_escaped_braces,
    input: r#"f"Literal braces: {{not an interpolation}}""#,
    formatted: Ok(r#"f"Literal braces: {{not an interpolation}}""#),
    // {{ and }} are escaped braces in format strings
);

test_case!(
    format_string_with_map,
    input: r#"f"Map: {{a: 1, b: 2}}""#,
    formatted: Ok(r#"f"Map: {{a: 1, b: 2}}""#),
    // Map-like syntax in escaped braces (literal text, not formatted)
);
