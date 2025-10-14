mod cases;

test_case!(
    true_literal,
    input: "true",
    formatted: Ok("true"),
);

test_case!(
    false_literal,
    input: "false",
    formatted: Ok("false"),
);

test_case!(
    boolean_with_spaces,
    input: "  true  ",
    formatted: Ok("true"),
);
