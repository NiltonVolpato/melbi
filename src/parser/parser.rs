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
    fn span_of(&self, expr: &Expr<'a>) -> Option<Span> {
        let p = &(expr as *const _);
        self.spans.borrow().get(p).copied()
    }

    fn parse_expr(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        match pair.as_rule() {
            Rule::main => self.parse_main(pair),
            Rule::expression => self.parse_expression(pair),
            Rule::array => self.parse_array(pair),
            Rule::integer => self.parse_integer(pair),
            Rule::float => self.parse_float(pair),
            Rule::boolean => self.parse_boolean(pair),
            Rule::string => self.parse_string(pair),
            Rule::bytes => self.parse_bytes(pair),
            Rule::format_string => self.parse_format_string(pair),
            Rule::record => self.parse_record(pair),
            Rule::map => self.parse_map(pair),
            Rule::grouped => self.parse_grouped(pair),
            Rule::ident => self.parse_ident(pair),
            _ => Err(pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: format!("Unhandled rule: {:?}", pair.as_rule()),
                },
                pair.as_span(),
            )),
        }
    }

    fn parse_main(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
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

    fn parse_expression(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        PRATT_PARSER
            .map_primary(|primary| self.parse_expr(primary))
            .map_prefix(|op, rhs| {
                let rhs_value = rhs?;
                let span = Span {
                    start: op.as_span().start(),
                    end: self.span_of(rhs_value).unwrap().end,
                };
                match op.as_rule() {
                    Rule::neg | Rule::not => self.parse_unary_op(op, rhs_value, span),
                    Rule::if_op => self.parse_if_expr(op, rhs_value, span),
                    Rule::lambda_op => self.parse_lambda_expr(op, rhs_value, span),
                    _ => unreachable!("Unknown prefix operator: {:?}", op.as_rule()),
                }
            })
            .map_infix(|lhs, op, rhs| {
                let lhs_expr = lhs?;
                let rhs_expr = rhs?;
                let span = Span {
                    start: self.span_of(lhs_expr).unwrap().start,
                    end: self.span_of(rhs_expr).unwrap().end,
                };
                match op.as_rule() {
                    Rule::add
                    | Rule::sub
                    | Rule::mul
                    | Rule::div
                    | Rule::pow
                    | Rule::and
                    | Rule::or => self.parse_binary_op(op, lhs_expr, rhs_expr, span),
                    Rule::otherwise_op => self.parse_otherwise_expr(lhs_expr, rhs_expr, span),
                    _ => unreachable!("Unknown binary operator: {:?}", op.as_rule()),
                }
            })
            .map_postfix(|lhs, op| {
                let lhs_expr = lhs?;
                let span = Span {
                    start: self.span_of(lhs_expr).unwrap().start,
                    end: op.as_span().end(),
                };
                match op.as_rule() {
                    Rule::call_op => self.parse_call_expr(lhs_expr, op, span),
                    Rule::index_op => self.parse_index_expr(lhs_expr, op, span),
                    Rule::field_op => self.parse_field_expr(lhs_expr, op, span),
                    Rule::cast_op => self.parse_cast_expr(lhs_expr, op, span),
                    Rule::where_op => self.parse_where_expr(lhs_expr, op, span),
                    _ => unreachable!("Unknown postfix operator: {:?}", op.as_rule()),
                }
            })
            .parse(pair.into_inner())
    }

    // Helper to allocate an expression with its span
    fn alloc_with_span(&self, expr: Expr<'a>, span: Span) -> &'a Expr<'a> {
        let node = self.arena.alloc(expr);
        self.spans.borrow_mut().insert(node.as_ptr(), span);
        node
    }

    // Prefix operators
    fn parse_unary_op(
        &self,
        op: Pair<Rule>,
        rhs: &'a Expr<'a>,
        span: Span,
    ) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let op_enum = match op.as_rule() {
            Rule::neg => UnaryOp::Neg,
            Rule::not => UnaryOp::Not,
            _ => unreachable!(),
        };
        Ok(self.alloc_with_span(
            Expr::Unary {
                op: op_enum,
                expr: rhs,
            },
            span,
        ))
    }

    fn parse_if_expr(
        &self,
        op: Pair<Rule>,
        else_branch: &'a Expr<'a>,
        span: Span,
    ) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let mut pairs = op.into_inner();
        let cond = self.parse_expr(pairs.next().unwrap())?;
        let then_branch = self.parse_expr(pairs.next().unwrap())?;
        Ok(self.alloc_with_span(
            Expr::If {
                cond,
                then_branch,
                else_branch,
            },
            span,
        ))
    }

    fn parse_lambda_expr(
        &self,
        op: Pair<Rule>,
        body: &'a Expr<'a>,
        span: Span,
    ) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let mut pairs = op.into_inner();
        let mut params = Vec::new();

        if let Some(params_pair) = pairs.peek() {
            if params_pair.as_rule() == Rule::lambda_params {
                let params_pair = pairs.next().unwrap();
                params = params_pair
                    .into_inner()
                    .map(|p| p.as_str().to_string())
                    .collect();
            }
        }

        Ok(self.alloc_with_span(Expr::Lambda { params, body }, span))
    }

    // Infix operators
    fn parse_binary_op(
        &self,
        op: Pair<Rule>,
        left: &'a Expr<'a>,
        right: &'a Expr<'a>,
        span: Span,
    ) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let op_enum = match op.as_rule() {
            Rule::add => BinaryOp::Add,
            Rule::sub => BinaryOp::Sub,
            Rule::mul => BinaryOp::Mul,
            Rule::div => BinaryOp::Div,
            Rule::pow => BinaryOp::Pow,
            Rule::and => BinaryOp::And,
            Rule::or => BinaryOp::Or,
            _ => unreachable!(),
        };
        Ok(self.alloc_with_span(
            Expr::Binary {
                op: op_enum,
                left,
                right,
            },
            span,
        ))
    }

    fn parse_otherwise_expr(
        &self,
        primary: &'a Expr<'a>,
        fallback: &'a Expr<'a>,
        span: Span,
    ) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        Ok(self.alloc_with_span(Expr::Otherwise { primary, fallback }, span))
    }

    // Postfix operators
    fn parse_call_expr(
        &self,
        callable: &'a Expr<'a>,
        op: Pair<Rule>,
        span: Span,
    ) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let args = op
            .into_inner()
            .map(|p| self.parse_expr(p))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.alloc_with_span(Expr::Call { callable, args }, span))
    }

    fn parse_index_expr(
        &self,
        value: &'a Expr<'a>,
        op: Pair<Rule>,
        span: Span,
    ) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let index = self.parse_expr(op.into_inner().next().unwrap())?;
        Ok(self.alloc_with_span(Expr::Index { value, index }, span))
    }

    fn parse_field_expr(
        &self,
        value: &'a Expr<'a>,
        op: Pair<Rule>,
        span: Span,
    ) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
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
        Ok(self.alloc_with_span(Expr::Field { value, field }, span))
    }

    fn parse_cast_expr(
        &self,
        expr: &'a Expr<'a>,
        op: Pair<Rule>,
        span: Span,
    ) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
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
        Ok(self.alloc_with_span(Expr::Cast { expr, ty }, span))
    }

    fn parse_where_expr(
        &self,
        expr: &'a Expr<'a>,
        op: Pair<Rule>,
        span: Span,
    ) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let bindings = op
            .into_inner()
            .map(|p| self.parse_binding(p))
            .collect::<Result<_, _>>()?;
        Ok(self.alloc_with_span(Expr::Where { expr, bindings }, span))
    }

    fn parse_array(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
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

    fn parse_integer(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
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

    fn parse_float(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
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

    fn parse_boolean(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
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

    fn parse_string(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
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

    fn parse_bytes(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
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

    fn parse_format_string(
        &self,
        pair: Pair<Rule>,
    ) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
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

    fn parse_record(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
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

    fn parse_map(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
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

    fn parse_grouped(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        self.parse_expr(pair.into_inner().next().unwrap())
    }

    fn parse_ident(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let pair_span = pair.as_span();
        let span = Span {
            start: pair_span.start(),
            end: pair_span.end(),
        };
        let node = self.arena.alloc(Expr::Ident(pair.as_str().to_string()));
        self.spans.borrow_mut().insert(node.as_ptr(), span);
        Ok(node)
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
        expr: root,
        spans: context.spans.into_inner(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{FormatSegment, TypeExpr};

    #[test]
    fn test_simple_binary_expr() {
        let arena = Bump::new();
        let input = "1 + 2";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Binary {
                op: BinaryOp::Add,
                left: arena.alloc(Expr::Literal(Literal::Int(1))),
                right: arena.alloc(Expr::Literal(Literal::Int(2))),
            }
        );

        assert_eq!(parsed.span_of(parsed.expr), Some(Span { start: 0, end: 5 }));
        let Expr::Binary { left, right, .. } = parsed.expr else {
            panic!("Expected binary expression, got {:?}", parsed.expr);
        };
        assert_eq!(parsed.span_of(left), Some(Span { start: 0, end: 1 }));
        assert_eq!(parsed.span_of(right), Some(Span { start: 4, end: 5 }));
    }

    #[test]
    fn test_if_expr() {
        let arena = Bump::new();
        let input = "if not false then false else true";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::If {
                cond: arena.alloc(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: arena.alloc(Expr::Literal(Literal::Bool(false))),
                }),
                then_branch: arena.alloc(Expr::Literal(Literal::Bool(false))),
                else_branch: arena.alloc(Expr::Literal(Literal::Bool(true))),
            }
        );

        assert_eq!(
            parsed.span_of(parsed.expr),
            Some(Span { start: 0, end: 33 })
        );
        let Expr::If {
            cond,
            then_branch,
            else_branch,
        } = parsed.expr
        else {
            panic!("Expected If expression");
        };
        assert_eq!(parsed.span_of(cond), Some(Span { start: 3, end: 12 }));
        assert_eq!(
            parsed.span_of(then_branch),
            Some(Span { start: 18, end: 23 })
        );
        assert_eq!(
            parsed.span_of(else_branch),
            Some(Span { start: 29, end: 33 })
        );
    }

    #[test]
    fn test_lambda_expr() {
        let arena = Bump::new();
        let input = "(x) => x + 1";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Lambda {
                params: vec!["x".to_string()],
                body: arena.alloc(Expr::Binary {
                    op: BinaryOp::Add,
                    left: arena.alloc(Expr::Ident("x".to_string())),
                    right: arena.alloc(Expr::Literal(Literal::Int(1))),
                }),
            }
        );

        assert_eq!(
            parsed.span_of(parsed.expr),
            Some(Span { start: 0, end: 12 })
        );
        let Expr::Lambda { body, .. } = parsed.expr else {
            panic!("Expected Lambda expression");
        };
        assert_eq!(parsed.span_of(body), Some(Span { start: 7, end: 12 }));
    }

    #[test]
    fn test_cast_expr() {
        let arena = Bump::new();
        let input = "1.0 as Integer";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Cast {
                expr: arena.alloc(Expr::Literal(Literal::Float(1.0))),
                ty: TypeExpr::Path("Integer".to_string()),
            }
        );

        assert_eq!(
            parsed.span_of(parsed.expr),
            Some(Span { start: 0, end: 14 })
        );
        let Expr::Cast { expr, .. } = parsed.expr else {
            panic!("Expected Cast expression");
        };
        assert_eq!(parsed.span_of(expr), Some(Span { start: 0, end: 3 }));
    }

    #[ignore = "parsing type names is not implemented yet"]
    #[test]
    fn test_cast_expr_type_names() {
        let arena = Bump::new();
        let input = "m as Map[String, Integer]";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Cast {
                expr: arena.alloc(Expr::Ident("m".to_string())),
                ty: TypeExpr::Parametrized {
                    path: "Map".to_string(),
                    params: vec![
                        TypeExpr::Path("String".to_string()),
                        TypeExpr::Path("Integer".to_string())
                    ]
                },
            }
        );

        assert_eq!(
            parsed.span_of(parsed.expr),
            Some(Span { start: 0, end: 25 })
        );
    }

    #[test]
    fn test_array_literal() {
        let arena = Bump::new();
        let input = "[1, 2, 3]";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Array(vec![
                arena.alloc(Expr::Literal(Literal::Int(1))),
                arena.alloc(Expr::Literal(Literal::Int(2))),
                arena.alloc(Expr::Literal(Literal::Int(3))),
            ])
        );

        assert_eq!(parsed.span_of(parsed.expr), Some(Span { start: 0, end: 9 }));
        let Expr::Array(items) = parsed.expr else {
            panic!("Expected Array expression");
        };
        assert_eq!(parsed.span_of(items[0]), Some(Span { start: 1, end: 2 }));
        assert_eq!(parsed.span_of(items[1]), Some(Span { start: 4, end: 5 }));
        assert_eq!(parsed.span_of(items[2]), Some(Span { start: 7, end: 8 }));
    }

    #[test]
    fn test_map_literal() {
        let arena = Bump::new();
        let input = "{a: 1, b: 2}";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Map(vec![
                (
                    arena.alloc(Expr::Ident("a".to_string())),
                    arena.alloc(Expr::Literal(Literal::Int(1))),
                ),
                (
                    arena.alloc(Expr::Ident("b".to_string())),
                    arena.alloc(Expr::Literal(Literal::Int(2))),
                ),
            ])
        );

        assert_eq!(
            parsed.span_of(parsed.expr),
            Some(Span { start: 0, end: 12 })
        );
        let Expr::Map(entries) = parsed.expr else {
            panic!("Expected Map expression");
        };
        assert_eq!(
            parsed.span_of(entries[0].0),
            Some(Span { start: 1, end: 2 })
        );
        assert_eq!(
            parsed.span_of(entries[0].1),
            Some(Span { start: 4, end: 5 })
        );
        assert_eq!(
            parsed.span_of(entries[1].0),
            Some(Span { start: 7, end: 8 })
        );
        assert_eq!(
            parsed.span_of(entries[1].1),
            Some(Span { start: 10, end: 11 })
        );
    }

    #[test]
    fn test_record_literal() {
        let arena = Bump::new();
        let input = "{ x = 1, y = 2 }";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Record(vec![
                ("x".to_string(), arena.alloc(Expr::Literal(Literal::Int(1)))),
                ("y".to_string(), arena.alloc(Expr::Literal(Literal::Int(2)))),
            ])
        );

        assert_eq!(
            parsed.span_of(parsed.expr),
            Some(Span { start: 0, end: 16 })
        );
        let Expr::Record(fields) = parsed.expr else {
            panic!("Expected Record expression");
        };
        assert_eq!(parsed.span_of(fields[0].1), Some(Span { start: 6, end: 7 }));
        assert_eq!(
            parsed.span_of(fields[1].1),
            Some(Span { start: 13, end: 14 })
        );
    }

    #[test]
    fn test_where_expr() {
        let arena = Bump::new();
        let input = "x + y where { x = 1, y = 2 }";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Where {
                expr: arena.alloc(Expr::Binary {
                    op: BinaryOp::Add,
                    left: arena.alloc(Expr::Ident("x".to_string())),
                    right: arena.alloc(Expr::Ident("y".to_string())),
                }),
                bindings: vec![
                    ("x".to_string(), arena.alloc(Expr::Literal(Literal::Int(1)))),
                    ("y".to_string(), arena.alloc(Expr::Literal(Literal::Int(2)))),
                ],
            }
        );

        assert_eq!(
            parsed.span_of(parsed.expr),
            Some(Span { start: 0, end: 28 })
        );
        let Expr::Where { expr, bindings } = parsed.expr else {
            panic!("Expected Where expression");
        };
        assert_eq!(parsed.span_of(expr), Some(Span { start: 0, end: 5 }));
        assert_eq!(
            parsed.span_of(bindings[0].1),
            Some(Span { start: 18, end: 19 })
        );
        assert_eq!(
            parsed.span_of(bindings[1].1),
            Some(Span { start: 25, end: 26 })
        );
    }

    #[test]
    fn test_otherwise_expr() {
        let arena = Bump::new();
        let input = "1 / 0 otherwise -1";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Otherwise {
                primary: arena.alloc(Expr::Binary {
                    op: BinaryOp::Div,
                    left: arena.alloc(Expr::Literal(Literal::Int(1))),
                    right: arena.alloc(Expr::Literal(Literal::Int(0))),
                }),
                fallback: arena.alloc(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: arena.alloc(Expr::Literal(Literal::Int(1))),
                }),
            }
        );

        assert_eq!(
            parsed.span_of(parsed.expr),
            Some(Span { start: 0, end: 18 })
        );
        let Expr::Otherwise { primary, fallback } = parsed.expr else {
            panic!("Expected Otherwise expression");
        };
        assert_eq!(parsed.span_of(primary), Some(Span { start: 0, end: 5 }));
        assert_eq!(parsed.span_of(fallback), Some(Span { start: 16, end: 18 }));
    }

    #[test]
    fn test_lambda_no_argument() {
        let arena = Bump::new();
        let input = "() => 42";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Lambda {
                params: vec![],
                body: arena.alloc(Expr::Literal(Literal::Int(42))),
            }
        );

        assert_eq!(parsed.span_of(parsed.expr), Some(Span { start: 0, end: 8 }));
        let Expr::Lambda { body, .. } = parsed.expr else {
            panic!("Expected Lambda expression");
        };
        assert_eq!(parsed.span_of(body), Some(Span { start: 6, end: 8 }));
    }

    #[test]
    fn test_empty_record_literal() {
        let arena = Bump::new();
        let input = "Record {}";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(*parsed.expr, Expr::Record(vec![]));

        assert_eq!(parsed.span_of(parsed.expr), Some(Span { start: 0, end: 9 }));
    }

    #[test]
    fn test_format_string_with_interpolation() {
        let arena = Bump::new();
        let input = "f\" Hello, {a + b} !\\n \"";
        let parsed = parse(&arena, input).expect("parse failed");

        assert_eq!(
            *parsed.expr,
            Expr::FormatStr(vec![
                FormatSegment::Text(" Hello, ".to_string()),
                FormatSegment::Expr(arena.alloc(Expr::Binary {
                    op: BinaryOp::Add,
                    left: arena.alloc(Expr::Ident("a".to_string())),
                    right: arena.alloc(Expr::Ident("b".to_string())),
                })),
                FormatSegment::Text(" !\\n ".to_string()),
            ])
        );

        assert_eq!(
            parsed.span_of(parsed.expr),
            Some(Span { start: 0, end: 23 })
        );
    }

    #[test]
    fn test_where_expr_with_bindings() {
        let arena = Bump::new();
        let input = "x + y where { x = 1, y = 2, }";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Where {
                expr: arena.alloc(Expr::Binary {
                    op: BinaryOp::Add,
                    left: arena.alloc(Expr::Ident("x".to_string())),
                    right: arena.alloc(Expr::Ident("y".to_string())),
                }),
                bindings: vec![
                    ("x".to_string(), arena.alloc(Expr::Literal(Literal::Int(1)))),
                    ("y".to_string(), arena.alloc(Expr::Literal(Literal::Int(2)))),
                ],
            }
        );

        assert_eq!(
            parsed.span_of(parsed.expr),
            Some(Span { start: 0, end: 29 })
        );
    }

    #[test]
    fn test_function_call() {
        let arena = Bump::new();
        let input = "foo(1, 2, 3)";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Call {
                callable: arena.alloc(Expr::Ident("foo".to_string())),
                args: vec![
                    arena.alloc(Expr::Literal(Literal::Int(1))),
                    arena.alloc(Expr::Literal(Literal::Int(2))),
                    arena.alloc(Expr::Literal(Literal::Int(3))),
                ],
            }
        );

        assert_eq!(
            parsed.span_of(parsed.expr),
            Some(Span { start: 0, end: 12 })
        );
        let Expr::Call { callable, args } = parsed.expr else {
            panic!("Expected Call expression");
        };
        assert_eq!(parsed.span_of(callable), Some(Span { start: 0, end: 3 }));
        assert_eq!(parsed.span_of(args[0]), Some(Span { start: 4, end: 5 }));
        assert_eq!(parsed.span_of(args[1]), Some(Span { start: 7, end: 8 }));
        assert_eq!(parsed.span_of(args[2]), Some(Span { start: 10, end: 11 }));
    }

    #[test]
    fn test_index_access() {
        let arena = Bump::new();
        let input = "arr[42]";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Index {
                value: arena.alloc(Expr::Ident("arr".to_string())),
                index: arena.alloc(Expr::Literal(Literal::Int(42))),
            }
        );

        assert_eq!(parsed.span_of(parsed.expr), Some(Span { start: 0, end: 7 }));
        let Expr::Index { value, index } = parsed.expr else {
            panic!("Expected Index expression");
        };
        assert_eq!(parsed.span_of(value), Some(Span { start: 0, end: 3 }));
        assert_eq!(parsed.span_of(index), Some(Span { start: 4, end: 6 }));
    }

    #[test]
    fn test_attr_access() {
        let arena = Bump::new();
        let input = "obj.field";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Field {
                value: arena.alloc(Expr::Ident("obj".to_string())),
                field: "field".to_string(),
            }
        );

        assert_eq!(parsed.span_of(parsed.expr), Some(Span { start: 0, end: 9 }));
        let Expr::Field { value, .. } = parsed.expr else {
            panic!("Expected Field expression");
        };
        assert_eq!(parsed.span_of(value), Some(Span { start: 0, end: 3 }));
    }

    #[test]
    fn test_string_literal() {
        let arena = Bump::new();
        let input = "\"Hello, world!\"";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Str("Hello, world!".to_string()))
        );

        assert_eq!(
            parsed.span_of(parsed.expr),
            Some(Span { start: 0, end: 15 })
        );
    }

    #[test]
    fn test_bytes_literal() {
        let arena = Bump::new();
        let input = "b\"Hello, bytes!\"";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Bytes(b"Hello, bytes!".to_vec()))
        );

        assert_eq!(
            parsed.span_of(parsed.expr),
            Some(Span { start: 0, end: 16 })
        );
    }

    #[test]
    fn test_single_quoted_string_literal() {
        let arena = Bump::new();
        let input = "'Hello, world!'";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Str("Hello, world!".to_string()))
        );

        assert_eq!(
            parsed.span_of(parsed.expr),
            Some(Span { start: 0, end: 15 })
        );
    }

    #[test]
    fn test_single_quoted_bytes_literal() {
        let arena = Bump::new();
        let input = "b'Hello, bytes!'";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Bytes(b"Hello, bytes!".to_vec()))
        );

        assert_eq!(
            parsed.span_of(parsed.expr),
            Some(Span { start: 0, end: 16 })
        );
    }

    #[test]
    fn test_integer_overflow() {
        let arena = Bump::new();
        let expr = "9223372036854775808"; // i64::MAX + 1
        let result = parse(&arena, expr);
        assert!(result.is_err(), "Expected failure parsing '{}'", expr);
    }
}
