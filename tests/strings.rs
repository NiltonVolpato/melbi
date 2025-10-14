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
    // Currently FAILS: outputs "" instead
);

test_case!(
    string_with_special_chars,
    input: r#""hello\nworld""#,
    formatted: Ok(r#""hello\nworld""#),
    // Currently FAILS: outputs "" instead
);

test_case!(
    format_string_empty,
    input: r#"f"""#,
    formatted: Ok(r#"f"""#),
);

test_case!(
    format_string_simple,
    input: r#"f"Hello {name}""#,
    formatted: Ok(r#"f"Hello {name}""#),
    // Currently FAILS if content is stripped
);

test_case!(
    format_string_multiple_interpolations,
    input: r#"f"{x} + {y} = {x+y}""#,
    formatted: Ok(r#"f"{x} + {y} = {x + y}""#),
    // Spacing around operators in interpolations
);

test_case!(
    format_string_complex,
    input: r#"f"Result: {result where{x=1,y=2,result=x+y}}""#,
    formatted: Ok(r#"f"Result: {result where { x = 1, y = 2, result = x + y }}""#),
);

test_case!(
    format_string_multiline_expression,
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
    // Multi-line expressions inside format strings should preserve formatting
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
    format_string_nested_braces,
    input: r#"f"Map: {[a=1,b=2]}""#,
    formatted: Ok(r#"f"Map: {[a = 1, b = 2]}""#),
    // Braces inside interpolations are part of the expression, not the string
);
