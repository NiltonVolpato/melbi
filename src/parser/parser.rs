use std::cell::RefCell;
use std::collections::HashMap;

use crate::ast::{BinaryOp, Expr, FormatSegment, Literal, ParsedExpr, Span, UnaryOp};
use bumpalo::Bump;
use lazy_static::lazy_static;
use pest::Parser;
use pest::iterators::Pair;
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest_derive::Parser;

// TODO: replace unwrap with map_err.

lazy_static! {
    // Note: precedence is defined lowest to highest.
    static ref PRATT_PARSER: PrattParser<Rule> = PrattParser::new()
        // (lowest precedence)
        // Lambda and where operators.
        .op(Op::prefix(Rule::lambda_op))                 // `(...) =>`
        .op(Op::postfix(Rule::where_op))                 // `where {}`

        // Fallback (error handling) operator.
        .op(Op::infix(Rule::otherwise_op, Assoc::Right)) // `otherwise`

        // Logical operators.
        .op(Op::prefix(Rule::if_op))                     // `if`
        .op(Op::infix(Rule::or, Assoc::Left))            // `or`
        .op(Op::infix(Rule::and, Assoc::Left))           // `and`
        .op(Op::prefix(Rule::not))                       // `not`

        // Arithmetic operators.
        .op(
            Op::infix(Rule::add, Assoc::Left) |
            Op::infix(Rule::sub, Assoc::Left)
        )                                               // `+`, `-`
        .op(
            Op::infix(Rule::mul, Assoc::Left) |
            Op::infix(Rule::div, Assoc::Left)
        )                                               // `*`, `/`
        .op(Op::prefix(Rule::neg))                       // `-`
        .op(Op::infix(Rule::pow, Assoc::Right))          // `^` (right-assoc))

        // Postfix operators.
        .op(Op::postfix(Rule::call_op))                  // `()`
        .op(Op::postfix(Rule::index_op))                 // `[]`
        .op(Op::postfix(Rule::field_op))                 // `.`  // XXX: add more precedence tests
        .op(Op::postfix(Rule::cast_op))                  // `as`
        // (highest precedence)
        ;
}

#[derive(Parser)]
#[grammar = "parser/expression.pest"]
pub struct ExpressionParser;

struct ParseContext<'a> {
    arena: &'a Bump,
    spans: RefCell<HashMap<*const Expr<'a>, Span>>,
}

impl<'a> ParseContext<'a> {
    pub fn span_of(&self, expr: &Expr<'a>) -> Option<Span> {
        let p = &(expr as *const _);
        self.spans.borrow().get(p).copied()
    }

    pub fn parse_expr(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        match pair.as_rule() {
            Rule::main => {
                let span = pair.as_span();
                self.parse_expr(pair.into_inner().next().ok_or_else(|| {
                    pest::error::Error::new_from_span(
                        pest::error::ErrorVariant::CustomError {
                            message: "missing expected pair in rule".to_string(),
                        },
                        span,
                    )
                })?)
            }

            Rule::expression => PRATT_PARSER
                .map_primary(|primary| self.parse_expr(primary))
                .map_prefix(|op, rhs| {
                    let rhs_value = rhs?;
                    let span = Span {
                        start: op.as_span().start(),
                        end: self.span_of(rhs_value).unwrap().end,
                    };
                    let op_enum = match op.as_rule() {
                        Rule::neg => UnaryOp::Neg,
                        Rule::not => UnaryOp::Not,
                        Rule::if_op => {
                            // Handle `if` expressions
                            let mut pairs = op.into_inner();
                            let cond = self.parse_expr(pairs.next().unwrap())?;
                            let then_branch = self.parse_expr(pairs.next().unwrap())?;
                            let else_branch = rhs_value;
                            let node = self.arena.alloc(Expr::If {
                                cond,
                                then_branch,
                                else_branch,
                            });
                            self.spans.borrow_mut().insert(node.as_ptr(), span);
                            return Ok(node);
                        }
                        Rule::lambda_op => {
                            // Handle lambda expressions
                            let mut pairs = op.into_inner();
                            let mut params = Vec::new();

                            // Peek at the next pair to determine if it has parameters
                            if let Some(params_pair) = pairs.peek() {
                                if params_pair.as_rule() == Rule::lambda_params {
                                    // Consume the parameters pair and parse the parameters
                                    let params_pair = pairs.next().unwrap();
                                    params = params_pair
                                        .into_inner()
                                        .map(|p| p.as_str().to_string())
                                        .collect();
                                }
                            }

                            // Parse the body of the lambda
                            let body = rhs_value;
                            let node = self.arena.alloc(Expr::Lambda { params, body });
                            self.spans.borrow_mut().insert(node.as_ptr(), span);
                            return Ok(node);
                        }
                        _ => unreachable!("Unknown prefix operator: {:?}", op.as_rule()),
                    };
                    let node = self.arena.alloc(Expr::Unary {
                        op: op_enum,
                        expr: rhs_value,
                    });
                    self.spans.borrow_mut().insert(node.as_ptr(), span);
                    Ok(node)
                })
                .map_infix(|lhs, op, rhs| {
                    let lhs_expr = lhs?;
                    let rhs_expr = rhs?;
                    let span = Span {
                        start: self.span_of(lhs_expr).unwrap().start,
                        end: self.span_of(rhs_expr).unwrap().end,
                    };
                    let op_enum = match op.as_rule() {
                        Rule::add => BinaryOp::Add,
                        Rule::sub => BinaryOp::Sub,
                        Rule::mul => BinaryOp::Mul,
                        Rule::div => BinaryOp::Div,
                        Rule::pow => BinaryOp::Pow,
                        Rule::and => BinaryOp::And,
                        Rule::or => BinaryOp::Or,
                        Rule::otherwise_op => {
                            let node = self.arena.alloc(Expr::Otherwise {
                                primary: lhs_expr,
                                fallback: rhs_expr,
                            });
                            self.spans.borrow_mut().insert(node.as_ptr(), span);
                            return Ok(node);
                        }
                        _ => unreachable!("Unknown binary operator: {:?}", op.as_rule()),
                    };
                    let node = self.arena.alloc(Expr::Binary {
                        op: op_enum,
                        left: lhs_expr,
                        right: rhs_expr,
                    });
                    self.spans.borrow_mut().insert(node.as_ptr(), span);
                    Ok(node)
                })
                .map_postfix(|lhs, op| match op.as_rule() {
                    Rule::call_op => {
                        let lhs_expr = lhs?;
                        let op_span = op.as_span();
                        let args = op
                            .into_inner()
                            .map(|p| self.parse_expr(p))
                            .collect::<Result<Vec<_>, _>>()?;
                        let span = Span {
                            start: self.span_of(lhs_expr).unwrap().start,
                            end: op_span.end(),
                        };
                        let node = self.arena.alloc(Expr::Call {
                            callable: lhs_expr,
                            args,
                        });
                        self.spans.borrow_mut().insert(node.as_ptr(), span);
                        Ok(node)
                    }
                    Rule::index_op => {
                        let lhs_expr = lhs?;
                        let op_span = op.as_span();
                        let index_expr = self.parse_expr(op.into_inner().next().unwrap())?;
                        let span = Span {
                            start: self.span_of(lhs_expr).unwrap().start,
                            end: op_span.end(),
                        };
                        let node = self.arena.alloc(Expr::Index {
                            value: lhs_expr,
                            index: index_expr,
                        });
                        self.spans.borrow_mut().insert(node.as_ptr(), span);
                        Ok(node)
                    }
                    Rule::field_op => {
                        let lhs_expr = lhs?;
                        let op_span = op.as_span();
                        let field = op
                            .into_inner()
                            .next()
                            .ok_or_else(|| {
                                pest::error::Error::new_from_span(
                                    pest::error::ErrorVariant::CustomError {
                                        message: "missing attribute ident".to_string(),
                                    },
                                    op_span,
                                )
                            })?
                            .as_str()
                            .to_string();
                        let span = Span {
                            start: self.span_of(lhs_expr).unwrap().start,
                            end: op_span.end(),
                        };
                        let node = self.arena.alloc(Expr::Field {
                            value: lhs_expr,
                            field,
                        });
                        self.spans.borrow_mut().insert(node.as_ptr(), span);
                        Ok(node)
                    }
                    Rule::cast_op => {
                        let lhs_expr = lhs?;
                        let op_span = op.as_span();
                        let ty = crate::ast::TypeExpr::Path(
                            op.into_inner()
                                .next()
                                .ok_or_else(|| {
                                    pest::error::Error::new_from_span(
                                        pest::error::ErrorVariant::CustomError {
                                            message: "missing type expression".to_string(),
                                        },
                                        op_span,
                                    )
                                })?
                                .as_str()
                                .trim()
                                .to_string(),
                        );
                        let span = Span {
                            start: self.span_of(lhs_expr).unwrap().start,
                            end: op_span.end(),
                        };
                        let node = self.arena.alloc(Expr::Cast { expr: lhs_expr, ty });
                        self.spans.borrow_mut().insert(node.as_ptr(), span);
                        Ok(node)
                    }
                    Rule::where_op => {
                        let lhs_expr = lhs?;
                        let op_span = op.as_span();
                        let bindings = op
                            .into_inner()
                            .map(|p| self.parse_binding(p))
                            .collect::<Result<_, _>>()?;
                        let span = Span {
                            start: self.span_of(lhs_expr).unwrap().start,
                            end: op_span.end(),
                        };
                        let node = self.arena.alloc(Expr::Where {
                            expr: lhs_expr,
                            bindings,
                        });
                        self.spans.borrow_mut().insert(node.as_ptr(), span);
                        Ok(node)
                    }
                    _ => unreachable!("Unknown postfix operator: {:?}", op.as_rule()),
                })
                .parse(pair.into_inner()),

            Rule::array => {
                let pair_span = pair.as_span();
                let items = pair
                    .into_inner()
                    .map(|p| self.parse_expr(p))
                    .collect::<Result<Vec<_>, _>>()?;
                let span = Span {
                    start: pair_span.start(),
                    end: pair_span.end(),
                };
                let node = self.arena.alloc(Expr::Array(items));
                self.spans.borrow_mut().insert(node.as_ptr(), span);
                Ok(node)
            }

            Rule::integer => {
                let pair_span = pair.as_span();
                let value = pair.as_str().parse().map_err(|_| {
                    pest::error::Error::new_from_span(
                        pest::error::ErrorVariant::CustomError {
                            message: "invalid integer literal".to_string(),
                        },
                        pair_span,
                    )
                })?;
                let span = Span {
                    start: pair_span.start(),
                    end: pair_span.end(),
                };
                let node = self.arena.alloc(Expr::Literal(Literal::Int(value)));
                self.spans.borrow_mut().insert(node.as_ptr(), span);
                Ok(node)
            }

            Rule::float => {
                let pair_span = pair.as_span();
                let value = pair.as_str().parse().map_err(|_| {
                    pest::error::Error::new_from_span(
                        pest::error::ErrorVariant::CustomError {
                            message: "invalid float literal".to_string(),
                        },
                        pair_span,
                    )
                })?;
                let span = Span {
                    start: pair_span.start(),
                    end: pair_span.end(),
                };
                let node = self.arena.alloc(Expr::Literal(Literal::Float(value)));
                self.spans.borrow_mut().insert(node.as_ptr(), span);
                Ok(node)
            }

            Rule::boolean => {
                let pair_span = pair.as_span();
                let value = match pair.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => {
                        return Err(pest::error::Error::new_from_span(
                            pest::error::ErrorVariant::CustomError {
                                message: "invalid boolean literal".to_string(),
                            },
                            pair_span,
                        ));
                    }
                };
                let span = Span {
                    start: pair_span.start(),
                    end: pair_span.end(),
                };
                let node = self.arena.alloc(Expr::Literal(Literal::Bool(value)));
                self.spans.borrow_mut().insert(node.as_ptr(), span);
                Ok(node)
            }

            Rule::string => {
                let pair_span = pair.as_span();
                let s = pair.as_str();
                let inner = &s[1..s.len() - 1];
                let span = Span {
                    start: pair_span.start(),
                    end: pair_span.end(),
                };
                let node = self
                    .arena
                    .alloc(Expr::Literal(Literal::Str(inner.to_string())));
                self.spans.borrow_mut().insert(node.as_ptr(), span);
                Ok(node)
            }

            Rule::bytes => {
                let pair_span = pair.as_span();
                let s = pair.as_str();
                let inner = &s[2..s.len() - 1];
                let span = Span {
                    start: pair_span.start(),
                    end: pair_span.end(),
                };
                let node = self
                    .arena
                    .alloc(Expr::Literal(Literal::Bytes(inner.as_bytes().to_vec())));
                self.spans.borrow_mut().insert(node.as_ptr(), span);
                Ok(node)
            }

            Rule::format_string => {
                let pair_span = pair.as_span();
                let segments = pair
                    .into_inner()
                    .map(|p| match p.as_rule() {
                        Rule::format_text => {
                            // TODO: handle escape sequences.
                            Ok(FormatSegment::Text(p.as_str().to_string()))
                        }
                        Rule::format_expr => {
                            let expr = self.parse_expr(p.into_inner().next().unwrap())?;
                            Ok(FormatSegment::Expr(expr))
                        }
                        _ => unreachable!("Unknown format string segment: {:?}", p.as_rule()),
                    })
                    .collect::<Result<_, _>>()?;
                let span = Span {
                    start: pair_span.start(),
                    end: pair_span.end(),
                };
                let node = self.arena.alloc(Expr::FormatStr(segments));
                self.spans.borrow_mut().insert(node.as_ptr(), span);
                Ok(node)
            }

            Rule::record => {
                let pair_span = pair.as_span();
                let fields = pair
                    .into_inner()
                    .map(|p| self.parse_binding(p))
                    .collect::<Result<_, _>>()?;
                let span = Span {
                    start: pair_span.start(),
                    end: pair_span.end(),
                };
                let node = self.arena.alloc(Expr::Record(fields));
                self.spans.borrow_mut().insert(node.as_ptr(), span);
                Ok(node)
            }

            Rule::map => {
                let pair_span = pair.as_span();
                let entries = pair
                    .into_inner()
                    .map(|p| self.parse_map_entry(p))
                    .collect::<Result<_, _>>()?;
                let span = Span {
                    start: pair_span.start(),
                    end: pair_span.end(),
                };
                let node = self.arena.alloc(Expr::Map(entries));
                self.spans.borrow_mut().insert(node.as_ptr(), span);
                Ok(node)
            }

            Rule::grouped => self.parse_expr(pair.into_inner().next().unwrap()),

            Rule::ident => {
                let pair_span = pair.as_span();
                let span = Span {
                    start: pair_span.start(),
                    end: pair_span.end(),
                };
                let node = self.arena.alloc(Expr::Ident(pair.as_str().to_string()));
                self.spans.borrow_mut().insert(node.as_ptr(), span);
                Ok(node)
            }

            _ => Err(pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: format!("Unhandled rule: {:?}", pair.as_rule()),
                },
                pair.as_span(),
            )),
        }
    }

    fn parse_binding(
        &self,
        pair: Pair<Rule>,
    ) -> Result<(String, &'a Expr<'a>), pest::error::Error<Rule>> {
        let span = pair.as_span();
        let mut inner = pair.into_inner();
        let name = inner
            .next()
            .ok_or_else(|| {
                pest::error::Error::new_from_span(
                    pest::error::ErrorVariant::CustomError {
                        message: "missing binding name".to_string(),
                    },
                    span,
                )
            })?
            .as_str()
            .to_string();
        let value = self.parse_expr(inner.next().ok_or_else(|| {
            pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: "missing binding value".to_string(),
                },
                span,
            )
        })?)?;
        Ok((name, value))
    }

    fn parse_map_entry(
        &self,
        pair: Pair<Rule>,
    ) -> Result<(&'a Expr<'a>, &'a Expr<'a>), pest::error::Error<Rule>> {
        let span = pair.as_span();
        let mut inner = pair.into_inner();
        let key = self.parse_expr(inner.next().ok_or_else(|| {
            pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: "missing map key".to_string(),
                },
                span,
            )
        })?)?;
        let value = self.parse_expr(inner.next().ok_or_else(|| {
            pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: "missing map value".to_string(),
                },
                span,
            )
        })?)?;
        Ok((key, value))
    }
}

pub fn parse<'a>(
    arena: &'a Bump,
    source: &str,
) -> Result<ParsedExpr<'a>, pest::error::Error<Rule>> {
    let mut pairs = ExpressionParser::parse(Rule::main, source)?;
    let pair = pairs.next().ok_or_else(|| {
        pest::error::Error::new_from_pos(
            pest::error::ErrorVariant::CustomError {
                message: "missing expected pair in rule".to_string(),
            },
            pest::Position::from_start(source),
        )
    })?;
    let context = ParseContext {
        arena,
        spans: RefCell::new(HashMap::new()),
    };
    let root = context.parse_expr(pair)?;
    Ok(ParsedExpr {
        source: source.to_string(),
        root,
        spans: context.spans.into_inner(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{FormatSegment, TypeExpr};

    #[test]
    fn test_simple_binary_expr() {
        let input = "1 + 2";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Binary {
                    op: BinaryOp::Add,
                    left: Box::new(Expr {
                        node: ExprNode::Literal(Literal::Int(1)),
                        span: Span { start: 0, end: 1 },
                    }),
                    right: Box::new(Expr {
                        node: ExprNode::Literal(Literal::Int(2)),
                        span: Span { start: 4, end: 5 },
                    }),
                },
                span: Span { start: 0, end: 5 },
            }
        );
    }

    #[test]
    fn test_if_expr() {
        let input = "if not false then false else true";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::If {
                    cond: Box::new(Expr {
                        node: ExprNode::Unary {
                            op: UnaryOp::Not,
                            expr: Box::new(Expr {
                                node: ExprNode::Literal(Literal::Bool(false)),
                                span: Span { start: 7, end: 12 },
                            }),
                        },
                        span: Span { start: 3, end: 12 },
                    }),
                    then_branch: Box::new(Expr {
                        node: ExprNode::Literal(Literal::Bool(false)),
                        span: Span { start: 18, end: 23 },
                    }),
                    else_branch: Box::new(Expr {
                        node: ExprNode::Literal(Literal::Bool(true)),
                        span: Span { start: 29, end: 33 },
                    }),
                },
                span: Span { start: 0, end: 33 },
            }
        );
    }

    #[test]
    fn test_lambda_expr() {
        let input = "(x) => x + 1";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Lambda {
                    params: vec!["x".to_string()],
                    body: Box::new(Expr {
                        node: ExprNode::Binary {
                            op: BinaryOp::Add,
                            left: Box::new(Expr {
                                node: ExprNode::Ident("x".to_string()),
                                span: Span { start: 7, end: 8 },
                            }),
                            right: Box::new(Expr {
                                node: ExprNode::Literal(Literal::Int(1)),
                                span: Span { start: 11, end: 12 },
                            }),
                        },
                        span: Span { start: 7, end: 12 },
                    }),
                },
                span: Span { start: 0, end: 12 },
            }
        );
    }

    #[test]
    fn test_cast_expr() {
        let input = "1.0 as Integer";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Cast {
                    expr: Box::new(Expr {
                        node: ExprNode::Literal(Literal::Float(1.0)),
                        span: Span { start: 0, end: 3 },
                    }),
                    ty: TypeExpr::Path("Integer".to_string()),
                },
                span: Span { start: 0, end: 14 },
            }
        );
    }

    #[ignore = "parsing type names is not implemented yet"]
    #[test]
    fn test_cast_expr_type_names() {
        let input = "m as Map[String, Integer]";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Cast {
                    expr: Box::new(Expr {
                        node: ExprNode::Ident("m".to_string()),
                        span: Span { start: 0, end: 1 },
                    }),
                    ty: TypeExpr::Parametrized {
                        path: "Map".to_string(),
                        params: vec![
                            TypeExpr::Path("String".to_string()),
                            TypeExpr::Path("Integer".to_string())
                        ]
                    },
                },
                span: Span { start: 0, end: 25 },
            }
        );
    }

    #[test]
    fn test_array_literal() {
        let input = "[1, 2, 3]";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Array(vec![
                    Expr {
                        node: ExprNode::Literal(Literal::Int(1)),
                        span: Span { start: 1, end: 2 },
                    },
                    Expr {
                        node: ExprNode::Literal(Literal::Int(2)),
                        span: Span { start: 4, end: 5 },
                    },
                    Expr {
                        node: ExprNode::Literal(Literal::Int(3)),
                        span: Span { start: 7, end: 8 },
                    },
                ]),
                span: Span { start: 0, end: 9 },
            }
        );
    }

    #[test]
    fn test_map_literal() {
        let input = "{a: 1, b: 2}";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Map(vec![
                    (
                        Expr {
                            node: ExprNode::Ident("a".to_string()),
                            span: Span { start: 1, end: 2 },
                        },
                        Expr {
                            node: ExprNode::Literal(Literal::Int(1)),
                            span: Span { start: 4, end: 5 },
                        }
                    ),
                    (
                        Expr {
                            node: ExprNode::Ident("b".to_string()),
                            span: Span { start: 7, end: 8 },
                        },
                        Expr {
                            node: ExprNode::Literal(Literal::Int(2)),
                            span: Span { start: 10, end: 11 },
                        }
                    ),
                ]),
                span: Span { start: 0, end: 12 },
            }
        );
    }

    #[test]
    fn test_record_literal() {
        let input = "{ x = 1, y = 2 }";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Record(vec![
                    (
                        "x".to_string(),
                        Expr {
                            node: ExprNode::Literal(Literal::Int(1)),
                            span: Span { start: 6, end: 7 },
                        }
                    ),
                    (
                        "y".to_string(),
                        Expr {
                            node: ExprNode::Literal(Literal::Int(2)),
                            span: Span { start: 13, end: 14 },
                        }
                    ),
                ]),
                span: Span { start: 0, end: 16 },
            }
        );
    }

    #[test]
    fn test_where_expr() {
        let input = "x + y where { x = 1, y = 2 }";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Where {
                    expr: Box::new(Expr {
                        node: ExprNode::Binary {
                            op: BinaryOp::Add,
                            left: Box::new(Expr {
                                node: ExprNode::Ident("x".to_string()),
                                span: Span { start: 0, end: 1 },
                            }),
                            right: Box::new(Expr {
                                node: ExprNode::Ident("y".to_string()),
                                span: Span { start: 4, end: 5 },
                            }),
                        },
                        span: Span { start: 0, end: 5 },
                    }),
                    bindings: vec![
                        (
                            "x".to_string(),
                            Expr {
                                node: ExprNode::Literal(Literal::Int(1)),
                                span: Span { start: 18, end: 19 },
                            }
                        ),
                        (
                            "y".to_string(),
                            Expr {
                                node: ExprNode::Literal(Literal::Int(2)),
                                span: Span { start: 25, end: 26 },
                            }
                        ),
                    ],
                },
                span: Span { start: 0, end: 28 },
            }
        );
    }

    #[test]
    fn test_otherwise_expr() {
        let input = "1 / 0 otherwise -1";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Otherwise {
                    primary: Box::new(Expr {
                        node: ExprNode::Binary {
                            op: BinaryOp::Div,
                            left: Box::new(Expr {
                                node: ExprNode::Literal(Literal::Int(1)),
                                span: Span { start: 0, end: 1 },
                            }),
                            right: Box::new(Expr {
                                node: ExprNode::Literal(Literal::Int(0)),
                                span: Span { start: 4, end: 5 },
                            }),
                        },
                        span: Span { start: 0, end: 5 },
                    }),
                    fallback: Box::new(Expr {
                        node: ExprNode::Unary {
                            op: UnaryOp::Neg,
                            expr: Box::new(Expr {
                                node: ExprNode::Literal(Literal::Int(1)),
                                span: Span { start: 17, end: 18 },
                            }),
                        },
                        span: Span { start: 16, end: 18 },
                    }),
                },
                span: Span { start: 0, end: 18 },
            }
        );
    }

    #[test]
    fn test_lambda_no_argument() {
        let input = "() => 42";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Lambda {
                    params: vec![],
                    body: Box::new(Expr {
                        node: ExprNode::Literal(Literal::Int(42)),
                        span: Span { start: 6, end: 8 },
                    }),
                },
                span: Span { start: 0, end: 8 },
            }
        );
    }

    #[test]
    fn test_empty_record_literal() {
        let input = "Record {}";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Record(vec![]),
                span: Span { start: 0, end: 9 },
            }
        );
    }

    #[test]
    fn test_format_string_with_interpolation() {
        let input = "f\" Hello, {a + b} !\\n \"";
        let parsed = parse(input).expect("parse failed");
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::FormatStr(vec![
                    FormatSegment::Text(" Hello, ".to_string()),
                    FormatSegment::Expr(Box::new(Expr {
                        node: ExprNode::Binary {
                            op: BinaryOp::Add,
                            left: Box::new(Expr {
                                node: ExprNode::Ident("a".to_string()),
                                span: Span { start: 11, end: 12 },
                            }),
                            right: Box::new(Expr {
                                node: ExprNode::Ident("b".to_string()),
                                span: Span { start: 15, end: 16 },
                            }),
                        },
                        span: Span { start: 11, end: 16 },
                    })),
                    FormatSegment::Text(" !\\n ".to_string()),
                ]),
                span: Span { start: 0, end: 23 },
            }
        );
    }

    #[test]
    fn test_where_expr_with_bindings() {
        let input = "x + y where { x = 1, y = 2, }";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Where {
                    expr: Box::new(Expr {
                        node: ExprNode::Binary {
                            op: BinaryOp::Add,
                            left: Box::new(Expr {
                                node: ExprNode::Ident("x".to_string()),
                                span: Span { start: 0, end: 1 },
                            }),
                            right: Box::new(Expr {
                                node: ExprNode::Ident("y".to_string()),
                                span: Span { start: 4, end: 5 },
                            }),
                        },
                        span: Span { start: 0, end: 5 },
                    }),
                    bindings: vec![
                        (
                            "x".to_string(),
                            Expr {
                                node: ExprNode::Literal(Literal::Int(1)),
                                span: Span { start: 18, end: 19 },
                            }
                        ),
                        (
                            "y".to_string(),
                            Expr {
                                node: ExprNode::Literal(Literal::Int(2)),
                                span: Span { start: 25, end: 26 },
                            }
                        ),
                    ],
                },
                span: Span { start: 0, end: 29 },
            }
        );
    }

    #[test]
    fn test_function_call() {
        let input = "foo(1, 2, 3)";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Call {
                    callable: Box::new(Expr {
                        node: ExprNode::Ident("foo".to_string()),
                        span: Span { start: 0, end: 3 },
                    }),
                    args: vec![
                        Expr {
                            node: ExprNode::Literal(Literal::Int(1)),
                            span: Span { start: 4, end: 5 },
                        },
                        Expr {
                            node: ExprNode::Literal(Literal::Int(2)),
                            span: Span { start: 7, end: 8 },
                        },
                        Expr {
                            node: ExprNode::Literal(Literal::Int(3)),
                            span: Span { start: 10, end: 11 },
                        },
                    ],
                },
                span: Span { start: 0, end: 12 },
            }
        );
    }

    #[test]
    fn test_index_access() {
        let input = "arr[42]";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Index {
                    value: Box::new(Expr {
                        node: ExprNode::Ident("arr".to_string()),
                        span: Span { start: 0, end: 3 },
                    }),
                    index: Box::new(Expr {
                        node: ExprNode::Literal(Literal::Int(42)),
                        span: Span { start: 4, end: 6 },
                    }),
                },
                span: Span { start: 0, end: 7 },
            }
        );
    }

    #[test]
    fn test_attr_access() {
        let input = "obj.field";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Field {
                    value: Box::new(Expr {
                        node: ExprNode::Ident("obj".to_string()),
                        span: Span { start: 0, end: 3 },
                    }),
                    field: "field".to_string(),
                },
                span: Span { start: 0, end: 9 },
            }
        );
    }

    #[test]
    fn test_string_literal() {
        let input = "\"Hello, world!\"";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Literal(Literal::Str("Hello, world!".to_string())),
                span: Span { start: 0, end: 15 },
            }
        );
    }

    #[test]
    fn test_bytes_literal() {
        let input = "b\"Hello, bytes!\"";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Literal(Literal::Bytes(b"Hello, bytes!".to_vec())),
                span: Span { start: 0, end: 16 },
            }
        );
    }

    #[test]
    fn test_single_quoted_string_literal() {
        let input = "'Hello, world!'";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Literal(Literal::Str("Hello, world!".to_string())),
                span: Span { start: 0, end: 15 },
            }
        );
    }

    #[test]
    fn test_single_quoted_bytes_literal() {
        let input = "b'Hello, bytes!'";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed.expr,
            Expr {
                node: ExprNode::Literal(Literal::Bytes(b"Hello, bytes!".to_vec())),
                span: Span { start: 0, end: 16 },
            }
        );
    }

    #[test]
    fn test_integer_overflow() {
        let expr = "9223372036854775808"; // i64::MAX + 1
        let result = parse(expr);
        assert!(result.is_err(), "Expected failure parsing '{}'", expr);
    }
}
