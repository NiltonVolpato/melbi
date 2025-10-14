mod cases;

test_case!(
    addition,
    input: "1+2",
    formatted: Ok("1 + 2"),
);

test_case!(
    subtraction,
    input: "5-3",
    formatted: Ok("5 - 3"),
);

test_case!(
    multiplication,
    input: "3*4",
    formatted: Ok("3 * 4"),
);

test_case!(
    division,
    input: "8/2",
    formatted: Ok("8 / 2"),
);

test_case!(
    power,
    input: "2^3",
    formatted: Ok("2 ^ 3"),
);

test_case!(
    logical_and,
    input: "true and false",
    formatted: Ok("true and false"),
);

test_case!(
    logical_or,
    input: "true or false",
    formatted: Ok("true or false"),
);

test_case!(
    otherwise_operator,
    input: "x otherwise 0",
    formatted: Ok("x otherwise 0"),
);

test_case!(
    negation,
    input: "-5",
    formatted: Ok("-5"),
);

test_case!(
    logical_not,
    input: "not true",
    formatted: Ok("not true"),
);

test_case!(
    complex_expression,
    input: "2+3*4-5/2",
    formatted: Ok("2 + 3 * 4 - 5 / 2"),
);

test_case!(
    parenthesized_expression,
    input: "(2+3)*(4-1)",
    formatted: Ok("(2 + 3) * (4 - 1)"),
);
