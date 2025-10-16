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

test_case!(
    simple_int,
    input: "42",
    ast: Ok(Expr::Literal(Literal::Int { value: 42, suffix: None })),
    formatted: Ok("42"),
    // Simple integer literal
);

test_case!(
    binary_int,
    input: "0b101010",
    ast: Ok(Expr::Literal(Literal::Int { value: 0b101010, suffix: None })),
    formatted: Ok("0b101010"),
    // Simple integer literal
);

test_case!(
    oct_int,
    input: "0o52",
    ast: Ok(Expr::Literal(Literal::Int { value: 0o52, suffix: None })),
    formatted: Ok("0o52"),
    // Simple integer literal
);

test_case!(
    hex_int,
    input: "0x2A",
    ast: Ok(Expr::Literal(Literal::Int { value: 0x2A, suffix: None })),
    formatted: Ok("0x2A"),
    // Simple integer literal
);

test_case!(
    invalid_binary_int,
    input: "0b3",
    ast: Err(_),
);

test_case!(
    int_with_leading_zeros_spaces,
    input: "  007  ",
    formatted: Ok("007"),
    // Trim whitespace, keep leading zeros
);

test_case!(
    negative_int,
    input: "-123",
    formatted: Ok("-123"),
    // Negative integers
);

test_case!(
    zero,
    input: "0",
    formatted: Ok("0"),
    // Zero
);

test_case!(
    large_int_with_underscores,
    input: "999_999_999_999_999",
    formatted: Ok("999_999_999_999_999"),
    // Large integers, keep underscores
);

test_case!(
    int_with_comment,
    input: "42  // answer to everything",
    formatted: Ok("42  // answer to everything"),
    // Comments after integers
);
