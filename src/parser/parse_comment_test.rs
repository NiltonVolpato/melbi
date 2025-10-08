use crate::{ast::*, parser::parse};
use bumpalo::Bump;

#[test]
fn test_top_comment() {
    let arena = Bump::new();
    let input = "// this is a comment\n1 + 2";
    let parsed = parse(&arena, input).unwrap();

    assert_eq!(
        *parsed.expr,
        Expr::Binary {
            op: BinaryOp::Add,
            left: arena.alloc(Expr::Literal(Literal::Int(1))),
            right: arena.alloc(Expr::Literal(Literal::Int(2))),
        }
    );

    assert_eq!(parsed.span_of(parsed.expr), Some(Span::new(21, 26)));
    let Expr::Binary { left, right, .. } = parsed.expr else {
        panic!("Expected binary expression, got {:?}", parsed.expr);
    };
    assert_eq!(parsed.span_of(left), Some(Span::new(21, 22)));
    assert_eq!(parsed.span_of(right), Some(Span::new(25, 26)));
}

#[test]
fn test_if_expr_comment() {
    let arena = Bump::new();
    let input = r#"
    if
    // comment1
    not false
    // comment2
    then "then"
    // comment3
    else "else"
    // comment4
    "#;
    let parsed = parse(&arena, input).unwrap();

    assert_eq!(
        *parsed.expr,
        Expr::If {
            cond: arena.alloc(Expr::Unary {
                op: UnaryOp::Not,
                expr: arena.alloc(Expr::Literal(Literal::Bool(false))),
            }),
            then_branch: arena.alloc(Expr::Literal(Literal::Str("then"))),
            else_branch: arena.alloc(Expr::Literal(Literal::Str("else"))),
        }
    );

    assert_eq!(
        parsed.comments,
        &[
            Span::new(12, 23),
            Span::new(42, 53),
            Span::new(74, 85),
            Span::new(106, 117),
        ]
    );

    assert_eq!(parsed.span_of(parsed.expr), Some(Span::new(5, 101)));
    let Expr::If {
        cond,
        then_branch,
        else_branch,
    } = parsed.expr
    else {
        panic!("Expected If expression");
    };
    assert_eq!(parsed.span_of(cond), Some(Span::new(28, 37)));
    assert_eq!(parsed.span_of(then_branch), Some(Span::new(63, 69)));
    assert_eq!(parsed.span_of(else_branch), Some(Span::new(95, 101)));
}
