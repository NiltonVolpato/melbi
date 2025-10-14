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
    negative_float,
    input: "-2.5",
    formatted: Ok("-2.5"),
);
