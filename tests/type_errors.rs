/*
 * Type Error Reporting Tests
 *
 * Tests that verify error messages, span accuracy, and provenance tracking
 * for various type checking scenarios.
 */

mod cases;

test_case! {
    name: if_condition_must_be_boolean,
    input: "if 1 then 0 else 0",
    error: { r#"
[E001] Error: Type mismatch: expected Bool, found Int
   ╭─[ <unknown>:1:4 ]
   │
 1 │ if 1 then 0 else 0
   │    ┬
   │    ╰── Type mismatch: expected Bool, found Int
   │
   │ Help: Condition of 'if' must be Bool
───╯
"#.trim_start() },
}

test_case! {
    name: otherwise_branch_type_mismatch,
    input: r#"123 + b + (1/0 otherwise "") where { b = 10 }"#,
    error: { r#"
[E001] Error: Type mismatch: expected Int, found Str
   ╭─[ <unknown>:1:26 ]
   │
 1 │ 123 + b + (1/0 otherwise "") where { b = 10 }
   │                          ─┬
   │                           ╰── Type mismatch: expected Int, found Str
   │
   │ Help: Types must match in this context
───╯
"#.trim_start() },
}

test_case! {
    name: undefined_variable,
    input: "x + 1",
    error: { r#"
[E002] Error: Undefined variable 'x'
   ╭─[ <unknown>:1:1 ]
   │
 1 │ x + 1
   │ ┬
   │ ╰── Undefined variable 'x'
   │
   │ Help: Make sure the variable is declared before use
───╯
"#.trim_start() },
}

test_case! {
    name: numeric_operation_on_bool,
    input: "true + false",
    error: { r#"
[E005] Error: Type 'Bool' does not implement Numeric
   ╭─[ <unknown>:1:1 ]
   │
 1 │ true + false
   │ ──────┬─────
   │       ╰─────── Type 'Bool' does not implement Numeric
───╯
"#.trim_start() },
}

test_case! {
    name: duplicate_binding_in_where,
    input: "x where { x = 1, x = 2 }",
    error: { r#"
[E016] Error: Duplicate binding name 'x'
   ╭─[ <unknown>:1:1 ]
   │
 1 │ x where { x = 1, x = 2 }
   │ ────────────┬───────────
   │             ╰───────────── Duplicate binding name 'x'
   │
   │ Help: Each binding in a where clause must have a unique name
───╯
"#.trim_start() },
}

test_case! {
    name: if_branch_type_mismatch,
    input: r#"if true then 1 else "hello""#,
    error: { r#"
[E001] Error: Type mismatch: expected Int, found Str
   ╭─[ <unknown>:1:21 ]
   │
 1 │ if true then 1 else "hello"
   │                     ───┬───
   │                        ╰───── Type mismatch: expected Int, found Str
   │
   │ Help: Types must match in this context
───╯
"#.trim_start() },
}

test_case! {
    name: unary_negation_on_bool,
    input: "-true",
    error: { r#"
[E005] Error: Type 'Bool' does not implement Numeric
   ╭─[ <unknown>:1:1 ]
   │
 1 │ -true
   │ ──┬──
   │   ╰──── Type 'Bool' does not implement Numeric
───╯
"#.trim_start() },
}

test_case! {
    name: logical_not_on_int,
    input: "not 42",
    error: { r#"
[E001] Error: Type mismatch: expected Bool, found Int
   ╭─[ <unknown>:1:5 ]
   │
 1 │ not 42
   │     ─┬
   │      ╰── Type mismatch: expected Bool, found Int
   │
   │ Help: Operand of 'not' must be Bool
───╯
"#.trim_start() },
}

test_case! {
    name: mixed_type_arithmetic,
    input: "1 + 2.5",
    error: { r#"
[E001] Error: Type mismatch: expected Int, found Float
   ╭─[ <unknown>:1:5 ]
   │
 1 │ 1 + 2.5
   │     ─┬─
   │      ╰─── Type mismatch: expected Int, found Float
   │
   │ Help: Types must match in this context
───╯
"#.trim_start() },
}

test_case! {
    name: duplicate_lambda_parameter,
    input: "(x, x) => x + 1",
    error: { r#"
[E015] Error: Duplicate parameter name 'x'
   ╭─[ <unknown>:1:1 ]
   │
 1 │ (x, x) => x + 1
   │ ───────┬───────
   │        ╰───────── Duplicate parameter name 'x'
   │
   │ Help: Each parameter must have a unique name
───╯
"#.trim_start() },
}

test_case! {
    name: ordering_comparison_on_bool,
    input: "lt(false, true) where { lt = (a, b) => a < b }",
    error: { r#"
[E005] Error: Type 'Bool' does not implement Ord
   ╭─[ <unknown>:1:40 ]
   │
 1 │ lt(false, true) where { lt = (a, b) => a < b }
   │                                        ──┬──
   │                                          ╰──── Type 'Bool' does not implement Ord
───╯
"#.trim_start() },
}

test_case! {
    name: numeric_operation_on_bool_polymorphic,
    input: "f(false, true) where { f = (a, b) => a + b }",
    error: { r#"
[E005] Error: Type 'Bool' does not implement Numeric
   ╭─[ <unknown>:1:38 ]
   │
 1 │ f(false, true) where { f = (a, b) => a + b }
   │                                      ──┬──
   │                                        ╰──── Type 'Bool' does not implement Numeric
───╯
"#.trim_start() },
}

test_case! {
    name: match_pattern_type_mismatch,
    input: r#"1 match { 1 -> 2, "foo" -> 10 }"#,
    error: { r#"
[E001] Error: Type mismatch: expected Int, found Str
   ╭─[ <unknown>:1:1 ]
   │
 1 │ 1 match { 1 -> 2, "foo" -> 10 }
   │ ───────────────┬───────────────
   │                ╰───────────────── Type mismatch: expected Int, found Str
   │
   │ Help: Pattern literal must match the type of the matched expression
───╯
"#.trim_start() },
}
