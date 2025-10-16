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
    formatted: r#""""#,
);

test_case!(
    simple_string,
    input: r#""hello""#,
    formatted: r#""hello""#,
    // Currently FAILS: outputs "" instead
);

test_case!(
    string_with_spaces,
    input: r#""hello world""#,
    formatted: r#""hello world""#,
);

test_case!(
    string_with_special_chars,
    input: r#""hello\nworld""#,
    formatted: r#""hello\nworld""#,
);

test_case!(
    empty_bytes,
    input: r#"b"""#,
    formatted: r#"b"""#,
);

test_case!(
    simple_bytes,
    input: r#"b"hello""#,
    formatted: r#"b"hello""#,
);

test_case!(
    bytes_with_escape,
    input: r#"b"\x48\x65\x6c\x6c\x6f""#,
    formatted: r#"b"\x48\x65\x6c\x6c\x6f""#,
);

test_case!(
    format_string_empty,
    input: r#"f"""#,
    formatted: r#"f"""#,
);

test_case!(
    format_string_simple,
    input: r#"f"Hello {name}""#,
    formatted: r#"f"Hello { name }""#,
);

test_case!(
    format_string_multiple_interpolations,
    input: r#"f"{x} + {y} = {x+y}""#,
    formatted: r#"f"{ x } + { y } = { x + y }""#,
    // Spacing around operators in interpolations AND around braces
);

test_case!(
    format_string_complex,
    input: r#"f"Result: {result where{x=1,y=2,result=x+y}}""#,
    formatted: r#"f"Result: { result where { x = 1, y = 2, result = x + y } }""#,
);

test_case!(
    format_string_multiline_expression,
    input: indoc! {r#"
        f"Hello {
            "Copilot"
        } from Melbi 🖖"
    "#},
    formatted: r#"
f"Hello {
    "Copilot"
} from Melbi 🖖"
"#.trim_start(),
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
    formatted: r#"
f"Result: {
    result where {
        x = 1,
        y = 2,
        result = x + y,
    }
}"
"#.trim_start(),
    // Format strings can contain arbitrarily complex multi-line expressions
);

test_case!(
    format_string_escaped_braces,
    input: r#"f"Literal braces: {{not an interpolation}}""#,
    formatted: r#"f"Literal braces: {{not an interpolation}}""#,
    // {{ and }} are escaped braces in format strings
);

test_case!(
    format_string_with_map,
    input: r#"f"Map: {{a: 1, b: 2}}""#,
    formatted: r#"f"Map: {{a: 1, b: 2}}""#,
    // Map-like syntax in escaped braces (literal text, not formatted)
);
