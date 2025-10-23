mod cases;

test_case! {
    name: around_if_condition,
    input: { "if (true or false) then 1 else 2" },
    formatted: { "if true or false then 1 else 2" },
}
