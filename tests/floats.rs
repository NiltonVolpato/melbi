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
    simple_float,
    input: "3.14",
    formatted: Ok("3.14"),
);

test_case!(
    float_with_leading_zero,
    input: "0.5",
    formatted: Ok("0.5"),
);

test_case!(
    float_scientific_notation,
    input: "1.5e10",
    formatted: Ok("1.5e10"),
);

test_case!(
    float_scientific_uppercase,
    input: "1.5E10",
    formatted: Ok("1.5E10"),
    // Preserve case of 'E' vs 'e'
);

test_case!(
    float_scientific_with_sign,
    input: "1.5e+10",
    formatted: Ok("1.5e+10"),
    // Preserve explicit positive sign
);

test_case!(
    float_scientific_negative,
    input: "1.5e-10",
    formatted: Ok("1.5e-10"),
    // Negative exponent
);

test_case!(
    negative_float,
    input: "-2.5",
    formatted: Ok("-2.5"),
);

test_case!(
    float_with_spaces,
    input: "  3.14  ",
    formatted: Ok("3.14"),
    // Trim whitespace around literals
);

test_case!(
    float_multiple_dots,
    input: "1.2.3",
    formatted: Err(_),
    // Invalid float syntax - parser error
);
