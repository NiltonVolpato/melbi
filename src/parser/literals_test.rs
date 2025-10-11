use super::parser::parse;
use crate::parser::{BinaryOp, Expr, Literal, UnaryOp};
use bumpalo::Bump;

#[test]
fn test_decimal_integers() {
    let arena = Bump::new();

    let cases = [
        ("0", 0),
        ("42", 42),
        ("123", 123),
        ("1_000", 1000),
        ("1_000_000", 1_000_000),
        ("1__000", 1000), // multiple underscores
        ("42_", 42),      // trailing underscore
    ];

    for (input, expected) in cases {
        let parsed = parse(&arena, input).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Int {
                value: expected,
                suffix: None,
            }),
            "Failed for input: {}",
            input
        );
    }
}

#[test]
fn test_binary_integers() {
    let arena = Bump::new();

    let cases = [
        ("0b0", 0),
        ("0b1", 1),
        ("0b1010", 0b1010),
        ("0b1111_0000", 0b1111_0000),
        ("0b_1010", 0b1010),   // underscore after prefix
        ("0b1010_", 0b1010),   // trailing underscore
        ("0b__1010", 0b1010),  // multiple underscores after prefix
        ("0b1_0_1_0", 0b1010), // underscores between digits
    ];

    for (input, expected) in cases {
        let parsed = parse(&arena, input).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Int {
                value: expected,
                suffix: None,
            }),
            "Failed for input: {}",
            input
        );
    }
}

#[test]
fn test_octal_integers() {
    let arena = Bump::new();

    let cases = [
        ("0o0", 0),
        ("0o7", 7),
        ("0o755", 0o755),
        ("0o7654_3210", 0o7654_3210),
        ("0o_755", 0o755), // underscore after prefix
        ("0o755_", 0o755), // trailing underscore
    ];

    for (input, expected) in cases {
        let parsed = parse(&arena, input).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Int {
                value: expected,
                suffix: None,
            }),
            "Failed for input: {}",
            input
        );
    }
}

#[test]
fn test_hexadecimal_integers() {
    let arena = Bump::new();

    let cases = [
        ("0x0", 0x0),
        ("0xA", 0xA),
        ("0x1A3F", 0x1A3F),
        ("0xDEADBEEF", 0xDEADBEEF_u64 as i64),
        ("0xDEAD_BEEF", 0xDEAD_BEEF_u64 as i64),
        ("0x_DEAD_BEEF", 0xDEAD_BEEF_u64 as i64), // underscore after prefix
        ("0xABCD_", 0xABCD),                      // trailing underscore
        ("0xabcd", 0xabcd),                       // lowercase
        ("0xAbCd", 0xAbCd),                       // mixed case
    ];

    for (input, expected) in cases {
        let parsed = parse(&arena, input).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Int {
                value: expected,
                suffix: None,
            }),
            "Failed for input: {}",
            input
        );
    }
}

#[test]
fn test_integers_with_suffix() {
    let arena = Bump::new();

    // Test with simple identifier suffix
    let parsed = parse(&arena, "42`kg`").unwrap();
    assert_eq!(
        *parsed.expr,
        Expr::Literal(Literal::Int {
            value: 42,
            suffix: Some(&Expr::Ident("kg")),
        })
    );

    // Test with complex suffix
    let parsed = parse(&arena, "100`m/s`").unwrap();
    assert_eq!(
        *parsed.expr,
        Expr::Literal(Literal::Int {
            value: 100,
            suffix: Some(&Expr::Binary {
                op: BinaryOp::Div,
                left: &Expr::Ident("m"),
                right: &Expr::Ident("s"),
            }),
        })
    );

    // Test different bases with suffix
    let parsed = parse(&arena, "0b1010`bits`").unwrap();
    assert_eq!(
        *parsed.expr,
        Expr::Literal(Literal::Int {
            value: 0b1010,
            suffix: Some(&Expr::Ident("bits")),
        })
    );

    // Test hex with suffix
    let parsed = parse(&arena, "0xFF`bytes`").unwrap();
    assert_eq!(
        *parsed.expr,
        Expr::Literal(Literal::Int {
            value: 0xFF,
            suffix: Some(&Expr::Ident("bytes")),
        })
    );

    // Test with negative exponent in suffix
    let parsed = parse(&arena, "440`s^-1`").unwrap();
    assert_eq!(
        *parsed.expr,
        Expr::Literal(Literal::Int {
            value: 440,
            suffix: Some(&Expr::Binary {
                op: BinaryOp::Pow,
                left: &Expr::Ident("s"),
                right: &Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: &Expr::Literal(Literal::Int {
                        value: 1,
                        suffix: None,
                    }),
                },
            }),
        })
    );

    // Test with complex unit expression
    let parsed = parse(&arena, "100`kg*m/s^2`").unwrap();
    assert_eq!(
        *parsed.expr,
        Expr::Literal(Literal::Int {
            value: 100,
            suffix: Some(&Expr::Binary {
                op: BinaryOp::Div,
                left: &Expr::Binary {
                    op: BinaryOp::Mul,
                    left: &Expr::Ident("kg"),
                    right: &Expr::Ident("m"),
                },
                right: &Expr::Binary {
                    op: BinaryOp::Pow,
                    left: &Expr::Ident("s"),
                    right: &Expr::Literal(Literal::Int {
                        value: 2,
                        suffix: None,
                    }),
                },
            }),
        })
    );
}

#[test]
fn test_float_literals() {
    let arena = Bump::new();

    let cases = [
        ("3.14", 3.14),
        ("3.", 3.0),
        (".5", 0.5),
        ("0.0", 0.0),
        ("1.0", 1.0),
        ("1_000.5", 1000.5),
        ("1_000.5_000", 1000.5),
        ("1.5_", 1.5), // trailing underscore
    ];

    for (input, expected) in cases {
        let parsed = parse(&arena, input).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Float {
                value: expected,
                suffix: None,
            }),
            "Failed for input: {}",
            input
        );
    }
}

#[test]
fn test_float_with_exponent() {
    let arena = Bump::new();

    let cases = [
        ("1e10", 1e10),
        ("1e-10", 1e-10),
        ("1e+10", 1e10),
        ("1.0e10", 1.0e10),
        ("3.14e-5", 3.14e-5),
        (".5e2", 0.5e2),
        ("6.022e23", 6.022e23),
        ("1.6E-19", 1.6e-19), // capital E
        ("1_000e10", 1000e10),
        ("1_000.0e+3", 1000.0e3),
        ("3.e10", 3.0e10), // no fractional part before exponent
    ];

    for (input, expected) in cases {
        let parsed = parse(&arena, input).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Float {
                value: expected,
                suffix: None,
            }),
            "Failed for input: {}",
            input
        );
    }
}

#[test]
fn test_floats_with_suffix() {
    let arena = Bump::new();

    // Test with simple identifier suffix
    let parsed = parse(&arena, "3.14`f32`").unwrap();
    assert_eq!(
        *parsed.expr,
        Expr::Literal(Literal::Float {
            value: 3.14,
            suffix: Some(&Expr::Ident("f32")),
        })
    );

    // Test with complex suffix
    let parsed = parse(&arena, "9.81`m/s^2`").unwrap();
    assert_eq!(
        *parsed.expr,
        Expr::Literal(Literal::Float {
            value: 9.81,
            suffix: Some(&Expr::Binary {
                op: BinaryOp::Div,
                left: &Expr::Ident("m"),
                right: &Expr::Binary {
                    op: BinaryOp::Pow,
                    left: &Expr::Ident("s"),
                    right: &Expr::Literal(Literal::Int {
                        value: 2,
                        suffix: None,
                    }),
                },
            }),
        })
    );

    // Test exponent with suffix
    let parsed = parse(&arena, "6.022e23`mol`").unwrap();
    assert_eq!(
        *parsed.expr,
        Expr::Literal(Literal::Float {
            value: 6.022e23,
            suffix: Some(&Expr::Ident("mol")),
        })
    );
}

#[test]
fn test_negative_numbers() {
    let arena = Bump::new();

    // Negative integers (parsed as unary negation)
    let parsed = parse(&arena, "-42").unwrap();
    assert_eq!(
        *parsed.expr,
        Expr::Unary {
            op: UnaryOp::Neg,
            expr: &Expr::Literal(Literal::Int {
                value: 42,
                suffix: None,
            }),
        }
    );

    // Negative floats
    let parsed = parse(&arena, "-3.14").unwrap();
    assert_eq!(
        *parsed.expr,
        Expr::Unary {
            op: UnaryOp::Neg,
            expr: &Expr::Literal(Literal::Float {
                value: 3.14,
                suffix: None,
            }),
        }
    );
}

#[test]
fn test_integer_overflow() {
    let arena = Bump::new();

    // 9223372036854775808 is i64::MAX + 1, should fail to parse
    let result = parse(&arena, "9223372036854775808");
    assert!(result.is_err(), "Expected overflow error for i64::MAX + 1");
}

#[test]
#[ignore] // TODO: needs special handling for i64::MIN
fn test_integer_min_value() {
    let arena = Bump::new();

    // -9223372036854775808 is i64::MIN, should work in the future
    // Currently fails because it parses as -(9223372036854775808)
    // and 9223372036854775808 overflows i64
    let parsed = parse(&arena, "-9223372036854775808").unwrap();
    assert_eq!(
        *parsed.expr,
        Expr::Unary {
            op: UnaryOp::Neg,
            expr: &Expr::Literal(Literal::Int {
                value: 9223372036854775808u64 as i64, // i64::MIN without the minus
                suffix: None,
            }),
        }
    );
}
