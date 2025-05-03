use pest::Parser;
use super::parser::ExpressionParser;
use super::parser::Rule;
use pest::error::Error;

#[test]
fn test_valid_expressions() -> Result<(), Error<Rule>> {
    let examples = [
        "1 + 2",
        "a * b + c",
        "if x then y else z",
        "x where {x = 1}",
        "foo(1, 2, 3)",
        "[1, 2, 3]",
        "{a: 1, b: 2}",
        "{x = 42}",
        "Record {}",
        "`0` + `some-name`",
        "1 as Integer",
        "\"abc\" as Bytes",
        "{ x = 1 } as Record[x: Integer]",
        "f\"Hello, {name}!\"",
        "'\\n'", // string escape
        "'\\u0041'",
        "b\"\\x41\"",
        "Record" // valid as identifier
    ];

    for expr in examples {
        ExpressionParser::parse(Rule::main, expr)
            .unwrap_or_else(|e| panic!("Failed to parse '{}': {}", expr, e));
    }

    Ok(())
}

#[test]
fn test_invalid_expressions() {
    let examples = [
        "1 +",
        "if x then else y",
        "{a: }",
        "Record[]",      // type, not value
        "1 as",          // missing type
        "1 as \"Int\"",  // invalid type expression
        "`",             // unterminated quoted ident
        "b\"\\u0041\"",  // invalid unicode escape in bytes
        "\"\\x41\"",     // invalid hex escape in string
    ];

    for expr in examples {
        assert!(
            ExpressionParser::parse(Rule::main, expr).is_err(),
            "Expected failure parsing '{}'", expr
        );
    }
}
