/*
 * Type Error Reporting Tests
 *
 * Tests that verify error messages, span accuracy, and provenance tracking
 * for various type checking scenarios.
 */

mod cases;

// TODO: All tests in this file need updating no_std, which affected error messages.

test_case! {
    name: if_condition_must_be_boolean,
    input: "if 1 then 0 else 0",
    #[ignore]
    error: { r#"
melbi_core::type_checking_error

  × While analyzing if expression
  ╰─▶ Type checking error
   ╭────
 1 │ if 1 then 0 else 0
   · ─────────┬────────
   ·          ╰── type mismatch here
   ╰────
  help: If condition must be boolean: expected Bool, got Int
"#.trim_start() },
}

test_case! {
    name: otherwise_branch_type_mismatch,
    input: r#"123 + b + (1/0 otherwise "") where { b = 10 }"#,
    #[ignore]
    error: { r#"
melbi_core::type_checking_error

  × While analyzing where expression
  ├─▶ While analyzing binary expression
  ├─▶ While analyzing 'otherwise' expression
  ├─▶ Type mismatch: Int vs Str
  ╰─▶ Type checking error
   ╭────
 1 │ 123 + b + (1/0 otherwise "") where { b = 10 }
   ·            ────────┬───────
   ·                    ╰── type mismatch here
   ╰────
  help: Primary and fallback branches must have compatible types
"#.trim_start() },
}

test_case! {
    name: undefined_variable,
    input: "x + 1",
    #[ignore]
    error: { r#"
melbi_core::type_checking_error

  × While analyzing binary expression
  ├─▶ While analyzing identifier
  ╰─▶ Type checking error
   ╭────
 1 │ x + 1
   · ┬
   · ╰── type mismatch here
   ╰────
  help: Undefined variable: 'x'
"#.trim_start() },
}

test_case! {
    name: numeric_operation_on_bool,
    input: "true + false",
    #[ignore]
    error: { r#"
melbi_core::type_checking_error

  × While analyzing binary expression
  ╰─▶ Type checking error
   ╭────
 1 │ true + false
   · ──────┬─────
   ·       ╰── type mismatch here
   ╰────
  help: left operand: expected Int or Float, got Bool
"#.trim_start() },
}

test_case! {
    name: duplicate_binding_in_where,
    input: "x where { x = 1, x = 2 }",
    #[ignore]
    error: { r#"
melbi_core::type_checking_error

  × While analyzing where expression
  ╰─▶ Type checking error
   ╭────
 1 │ x where { x = 1, x = 2 }
   · ────────────┬───────────
   ·             ╰── type mismatch here
   ╰────
  help: Duplicate binding name 'x'
"#.trim_start() },
}

test_case! {
    name: if_branch_type_mismatch,
    input: r#"if true then 1 else "hello""#,
    #[ignore]
    error: { r#"
melbi_core::type_checking_error

  × While analyzing if expression
  ├─▶ Type mismatch: Int vs Str
  ╰─▶ Type checking error
   ╭────
 1 │ if true then 1 else "hello"
   · ─────────────┬─────────────
   ·              ╰── type mismatch here
   ╰────
  help: Branches have incompatible types
"#.trim_start() },
}

test_case! {
    name: unary_negation_on_bool,
    input: "-true",
    #[ignore]
    error: { r#"
melbi_core::type_checking_error

  × While analyzing unary expression
  ╰─▶ Type checking error
   ╭────
 1 │ -true
   · ──┬──
   ·   ╰── type mismatch here
   ╰────
  help: unary negation: expected Int or Float, got Bool
"#.trim_start() },
}

test_case! {
    name: logical_not_on_int,
    input: "not 42",
    #[ignore]
    error: { r#"
melbi_core::type_checking_error

  × While analyzing unary expression
  ╰─▶ Type checking error
   ╭────
 1 │ not 42
   · ───┬──
   ·    ╰── type mismatch here
   ╰────
  help: unary not: expected Bool, got Int
"#.trim_start() },
}

test_case! {
    name: mixed_type_arithmetic,
    input: "1 + 2.5",
    #[ignore]
    error: { r#"
melbi_core::type_checking_error

  × While analyzing binary expression
  ╰─▶ Type checking error
   ╭────
 1 │ 1 + 2.5
   · ───┬───
   ·    ╰── type mismatch here
   ╰────
  help: operands must have same type: expected Float, got Int
"#.trim_start() },
}

test_case! {
    name: duplicate_lambda_parameter,
    input: "(x, x) => x + 1",
    #[ignore]
    error: { r#"
melbi_core::type_checking_error

  × While analyzing lambda expression
  ╰─▶ Type checking error
   ╭────
 1 │ (x, x) => x + 1
   · ───────┬───────
   ·        ╰── type mismatch here
   ╰────
  help: Duplicate parameter name 'x'
"#.trim_start() },
}

test_case! {
    name: ordering_comparison_on_bool,
    input: "lt(false, true) where { lt = (a, b) => a < b }",
    #[ignore]
    error: { r#"
melbi_core::type_checking_error

  × While analyzing where expression
  ├─▶ While analyzing comparison expression
  ╰─▶ Type checking error
   ╭────
 1 │ lt(false, true) where { lt = (a, b) => a < b }
   ·                                        ──┬──
   ·                                          ╰── type mismatch here
   ╰────
  help: Type Bool does not satisfy constraint Ord (required for comparison operations (<, >, <=, >=))
"#.trim_start() },
}

test_case! {
    name: numeric_operation_on_bool_polymorphic,
    input: "f(false, true) where { f = (a, b) => a + b }",
    #[ignore]
    error: { r#"
melbi_core::type_checking_error

  × While analyzing where expression
  ├─▶ While analyzing binary expression
  ╰─▶ Type checking error
   ╭────
 1 │ f(false, true) where { f = (a, b) => a + b }
   ·                                      ──┬──
   ·                                        ╰── type mismatch here
   ╰────
  help: Type Bool does not satisfy constraint Numeric (required for arithmetic operations (+, -, *, /, ^))
"#.trim_start() },
}
