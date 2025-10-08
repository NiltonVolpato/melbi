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

// Helper to find a comment span by its text content.
fn find_comment_span<'a>(parsed: &ParsedExpr<'a>, text: &str) -> Span {
    *parsed
        .comments
        .iter()
        .find(|&&span| parsed.snippet(span) == text)
        .unwrap_or_else(|| panic!("Comment with text '{}' not found", text))
}

// Helper to assert that spans are sorted and non-overlapping.
fn assert_sorted_and_non_overlapping(spans: &[Span]) {
    if spans.is_empty() {
        return;
    }
    for i in 0..spans.len() - 1 {
        let current = spans[i];
        let next = spans[i + 1];
        assert!(
            current.end <= next.start,
            "Spans are not sorted or are overlapping: {:?} is not before {:?}",
            current,
            next
        );
    }
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

    // Add snippet assertions for clarity
    assert_eq!(parsed.snippet(parsed.comments[0]), "// comment1");
    assert_eq!(parsed.snippet(parsed.comments[1]), "// comment2");
    assert_eq!(parsed.snippet(parsed.comments[2]), "// comment3");
    assert_eq!(parsed.snippet(parsed.comments[3]), "// comment4");

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

// New test using the relative assertion strategy
#[test]
fn test_if_expr_comment_relative_order() {
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

    let Expr::If {
        cond,
        then_branch,
        else_branch,
    } = parsed.expr
    else {
        panic!("Expected If expression");
    };

    let elements = &[
        find_comment_span(&parsed, "// comment1"),
        parsed.span_of(cond).unwrap(),
        find_comment_span(&parsed, "// comment2"),
        parsed.span_of(then_branch).unwrap(),
        find_comment_span(&parsed, "// comment3"),
        parsed.span_of(else_branch).unwrap(),
        find_comment_span(&parsed, "// comment4"),
    ];

    assert_sorted_and_non_overlapping(elements);
}

#[test]
fn test_multiple_and_inline_comments() {
    let arena = Bump::new();
    let input = r#"
    // comment 1
    // comment 2
    1 + 2 // inline comment
    // comment 3
    "#;
    let parsed = parse(&arena, input).unwrap();

    let Expr::Binary { left, right, .. } = parsed.expr else {
        panic!("Expected Binary expression");
    };

    let elements = &[
        find_comment_span(&parsed, "// comment 1"),
        find_comment_span(&parsed, "// comment 2"),
        parsed.span_of(left).unwrap(),
        parsed.span_of(right).unwrap(),
        find_comment_span(&parsed, "// inline comment"),
        find_comment_span(&parsed, "// comment 3"),
    ];

    assert_sorted_and_non_overlapping(elements);
}

#[test]
fn test_where_clause_comments() {
    let arena = Bump::new();
    let input = r#"
    a + b
    where {
        a = 1,  // comment for a
        // comment for b
        b = 2,
    }
    "#;
    let parsed = parse(&arena, input).unwrap();

    let Expr::Where { expr, bindings } = parsed.expr else {
        panic!("Expected Where expression");
    };

    let Expr::Binary { .. } = expr else {
        panic!("Expected inner Binary expression");
    };

    let elements = &[
        parsed.span_of(expr).unwrap(),
        parsed.span_of(bindings[0].1).unwrap(),
        find_comment_span(&parsed, "// comment for a"),
        find_comment_span(&parsed, "// comment for b"),
        parsed.span_of(bindings[1].1).unwrap(),
    ];

    assert_sorted_and_non_overlapping(elements);
}

#[test]
fn test_array_and_call_comments() {
    let arena = Bump::new();
    let input = r#"
    foo(
        [
            1, // one
            // two
            2
        ],
        // final arg
        3 // three
    )
    "#;
    let parsed = parse(&arena, input).unwrap();

    let Expr::Call { callable, args } = parsed.expr else {
        panic!("Expected Call expression");
    };

    let Expr::Array(items) = args[0] else {
        panic!("Expected Array expression");
    };

    let elements = &[
        parsed.span_of(callable).unwrap(),
        parsed.span_of(items[0]).unwrap(),
        find_comment_span(&parsed, "// one"),
        find_comment_span(&parsed, "// two"),
        parsed.span_of(items[1]).unwrap(),
        find_comment_span(&parsed, "// final arg"),
        parsed.span_of(args[1]).unwrap(),
        find_comment_span(&parsed, "// three"),
    ];

    assert_sorted_and_non_overlapping(elements);
}

#[test]
fn test_format_string_comment() {
    let arena = Bump::new();
    let input = r#"
f"Result: {
    if approved  // whether subject passed
    then "PASS"
    else "FAIL"
}!"
"#;
    let parsed = parse(&arena, input).unwrap();

    // Destructure to get to the 'if' expression inside the format string
    let Expr::FormatStr(segments) = parsed.expr else {
        panic!("Expected FormatStr expression");
    };
    let FormatSegment::Expr(if_expr) = segments[1] else {
        panic!("Expected FormatSegment::Expr");
    };
    let Expr::If {
        cond,
        then_branch,
        else_branch,
    } = if_expr
    else {
        panic!("Expected If expression");
    };

    let elements = &[
        parsed.span_of(cond).unwrap(),
        find_comment_span(&parsed, "// whether subject passed"),
        parsed.span_of(then_branch).unwrap(),
        parsed.span_of(else_branch).unwrap(),
    ];

    assert_sorted_and_non_overlapping(elements);
}
