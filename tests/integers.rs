mod cases;

test_case!(
    simple_int,
    input: "42",
    ast: Ok(Expr::Literal(Literal::Int { value: 42, suffix: None })),
);

test_case!(
    invalid_binary_int,
    input: "0b3",
    ast: Err(_),
);
