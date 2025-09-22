use crate::ast::{BinaryOp, Expr, FormatSegment, Literal, UnaryOp};
use lazy_static::lazy_static;
use pest::Parser;
use pest::iterators::Pair;
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest_derive::Parser;

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

pub fn parse_expr(pair: Pair<Rule>) -> Result<Expr, pest::error::Error<Rule>> {
    match pair.as_rule() {
        Rule::main => parse_expr(pair.into_inner().next().ok_or_else(|| {
            pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: "missing expected pair in rule".to_string(),
                },
                pair.as_span(),
            )
        })?),

        Rule::expression => PRATT_PARSER
            .map_primary(|primary| parse_expr(primary))
            .map_prefix(|op, rhs| {
                let op = match op.as_rule() {
                    Rule::neg => UnaryOp::Neg,
                    Rule::not => UnaryOp::Not,
                    Rule::if_op => {
                        // Handle `if` expressions
                        let mut pairs = op.into_inner();
                        let cond = parse_expr(pairs.next().unwrap())?;
                        let then_branch = parse_expr(pairs.next().unwrap())?;
                        return Ok(Expr::If {
                            cond: Box::new(cond),
                            then_branch: Box::new(then_branch),
                            else_branch: Box::new(rhs?),
                        });
                    }
                    Rule::lambda_op => {
                        // Handle lambda expressions
                        let mut pairs = op.into_inner();
                        let mut param_idents = Vec::new();

                        // Peek at the next pair to determine if it has parameters
                        if let Some(params_pair) = pairs.peek() {
                            if params_pair.as_rule() == Rule::lambda_params {
                                // Consume the parameters pair and parse the parameters
                                let params_pair = pairs.next().unwrap();
                                param_idents = params_pair
                                    .into_inner()
                                    .map(|p| p.as_str().to_string())
                                    .collect();
                            }
                        }

                        // Parse the body of the lambda
                        let body = parse_expr(pairs.next().unwrap())?;
                        return Ok(Expr::Lambda {
                            params: param_idents,
                            body: Box::new(body),
                        });
                    }
                    _ => unreachable!("Unknown prefix operator: {:?}", op.as_rule()),
                };
                Ok(Expr::Unary {
                    op,
                    expr: Box::new(rhs?),
                })
            })
            .map_infix(|lhs, op, rhs| {
                let op = match op.as_rule() {
                    Rule::add => BinaryOp::Add,
                    Rule::sub => BinaryOp::Sub,
                    Rule::mul => BinaryOp::Mul,
                    Rule::div => BinaryOp::Div,
                    Rule::pow => BinaryOp::Pow,
                    Rule::and => BinaryOp::And,
                    Rule::or => BinaryOp::Or,
                    Rule::otherwise_op => {
                        return Ok(Expr::Otherwise {
                            primary: Box::new(lhs?),
                            fallback: Box::new(rhs?),
                        });
                    }
                    _ => unreachable!("Unknown binary operator: {:?}", op.as_rule()),
                };
                Ok(Expr::Binary {
                    op,
                    left: Box::new(lhs?),
                    right: Box::new(rhs?),
                })
            })
            .map_postfix(|lhs, op| match op.as_rule() {
                Rule::call_op => {
                    let args = op.into_inner().map(parse_expr).collect::<Result<_, _>>()?;
                    Ok(Expr::Call {
                        callable: Box::new(lhs?),
                        args,
                    })
                }
                Rule::index_op => {
                    let index_expr = parse_expr(op.into_inner().next().unwrap())?;
                    Ok(Expr::Index {
                        value: Box::new(lhs?),
                        index: Box::new(index_expr),
                    })
                }
                Rule::field_op => {
                    let field = op
                        .into_inner()
                        .next()
                        .ok_or_else(|| {
                            pest::error::Error::new_from_span(
                                pest::error::ErrorVariant::CustomError {
                                    message: "missing attribute ident".to_string(),
                                },
                                op.as_span(),
                            )
                        })?
                        .as_str()
                        .to_string();
                    Ok(Expr::Field {
                        value: Box::new(lhs?),
                        field,
                    })
                }
                Rule::cast_op => {
                    unimplemented!("Type expression parsing not implemented yet");
                    // let ty = parse_type_expr(op.into_inner().next().unwrap())?;
                    // Ok(Expr::Cast {
                    //     expr: Box::new(lhs?),
                    //     ty,
                    // })
                }
                Rule::where_op => {
                    let bindings = op
                        .into_inner()
                        .map(parse_binding)
                        .collect::<Result<_, _>>()?;
                    Ok(Expr::Where {
                        expr: Box::new(lhs?),
                        bindings,
                    })
                }
                _ => unreachable!("Unknown postfix operator: {:?}", op.as_rule()),
            })
            .parse(pair.into_inner()),

        Rule::array => {
            let items = pair
                .into_inner()
                .map(parse_expr)
                .collect::<Result<_, _>>()?;
            Ok(Expr::Array(items))
        }

        Rule::integer => {
            let value = pair.as_str().parse().map_err(|_| {
                pest::error::Error::new_from_span(
                    pest::error::ErrorVariant::CustomError {
                        message: "invalid integer literal".to_string(),
                    },
                    pair.as_span(),
                )
            })?;
            Ok(Expr::Literal(Literal::Int(value)))
        }

        Rule::float => {
            let value = pair.as_str().parse().map_err(|_| {
                pest::error::Error::new_from_span(
                    pest::error::ErrorVariant::CustomError {
                        message: "invalid float literal".to_string(),
                    },
                    pair.as_span(),
                )
            })?;
            Ok(Expr::Literal(Literal::Float(value)))
        }

        Rule::boolean => {
            let value = match pair.as_str() {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(pest::error::Error::new_from_span(
                        pest::error::ErrorVariant::CustomError {
                            message: "invalid boolean literal".to_string(),
                        },
                        pair.as_span(),
                    ));
                }
            };
            Ok(Expr::Literal(Literal::Bool(value)))
        }

        Rule::string => {
            let s = pair.as_str();
            let inner = &s[1..s.len() - 1];
            Ok(Expr::Literal(Literal::Str(inner.to_string())))
        }

        Rule::bytes => {
            let s = pair.as_str();
            let inner = &s[2..s.len() - 1];
            Ok(Expr::Literal(Literal::Bytes(inner.as_bytes().to_vec())))
        }

        Rule::grouped => parse_expr(pair.into_inner().next().unwrap()),

        Rule::ident => Ok(Expr::Ident(pair.as_str().to_string())),

        _ => Err(pest::error::Error::new_from_span(
            pest::error::ErrorVariant::CustomError {
                message: format!("Unhandled rule: {:?}", pair.as_rule()),
            },
            pair.as_span(),
        )),
    }
}

fn parse_binding(pair: Pair<Rule>) -> Result<(String, Expr), pest::error::Error<Rule>> {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or_else(|| {
            pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: "missing binding name".to_string(),
                },
                pair.as_span(),
            )
        })?
        .as_str()
        .to_string();
    let value = parse_expr(inner.next().ok_or_else(|| {
        pest::error::Error::new_from_span(
            pest::error::ErrorVariant::CustomError {
                message: "missing binding value".to_string(),
            },
            pair.as_span(),
        )
    })?)?;
    Ok((name, value))
}

fn parse_map_entry(pair: Pair<Rule>) -> Result<(Expr, Expr), pest::error::Error<Rule>> {
    let mut inner = pair.into_inner();
    let key = parse_expr(inner.next().ok_or_else(|| {
        pest::error::Error::new_from_span(
            pest::error::ErrorVariant::CustomError {
                message: "missing map key".to_string(),
            },
            pair.as_span(),
        )
    })?)?;
    let value = parse_expr(inner.next().ok_or_else(|| {
        pest::error::Error::new_from_span(
            pest::error::ErrorVariant::CustomError {
                message: "missing map value".to_string(),
            },
            pair.as_span(),
        )
    })?)?;
    Ok((key, value))
}

pub fn parse(source: &str) -> Result<Expr, pest::error::Error<Rule>> {
    let mut pairs = ExpressionParser::parse(Rule::main, source)?;
    let pair = pairs.next().ok_or_else(|| {
        pest::error::Error::new_from_span(
            pest::error::ErrorVariant::CustomError {
                message: "missing expected pair in rule".to_string(),
            },
            pairs.as_span(),
        )
    })?;
    parse_expr(pair)
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
            parsed,
            Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expr::Literal(Literal::Int(1))),
                right: Box::new(Expr::Literal(Literal::Int(2))),
            }
        );
    }

    #[test]
    fn test_if_expr() {
        let input = "if not false then false else true";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::If {
                cond: Box::new(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(Expr::Literal(Literal::Bool(false))),
                }),
                then_branch: Box::new(Expr::Literal(Literal::Bool(false))),
                else_branch: Box::new(Expr::Literal(Literal::Bool(true))),
            }
        );
    }

    #[test]
    fn test_lambda_expr() {
        let input = "(x) => x + 1";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Lambda {
                params: vec!["x".to_string()],
                body: Box::new(Expr::Binary {
                    op: BinaryOp::Add,
                    left: Box::new(Expr::Ident("x".to_string())),
                    right: Box::new(Expr::Literal(Literal::Int(1))),
                }),
            }
        );
    }

    #[test]
    fn test_cast_expr() {
        let input = "1.0 as Integer";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Cast {
                expr: Box::new(Expr::Literal(Literal::Float(1.0))),
                ty: TypeExpr::Path("Integer".to_string()),
            }
        );
    }

    #[ignore = "parsing type names is not implemented yet"]
    #[test]
    fn test_cast_expr_type_names() {
        let input = "m as Map[String, Integer]";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Cast {
                expr: Box::new(Expr::Ident("m".to_string())),
                ty: TypeExpr::Parametrized {
                    path: "Map".to_string(),
                    params: vec![
                        TypeExpr::Path("String".to_string()),
                        TypeExpr::Path("Integer".to_string())
                    ]
                },
            }
        );
    }

    #[test]
    fn test_array_literal() {
        let input = "[1, 2, 3]";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Array(vec![
                Expr::Literal(Literal::Int(1)),
                Expr::Literal(Literal::Int(2)),
                Expr::Literal(Literal::Int(3)),
            ])
        );
    }

    #[test]
    fn test_map_literal() {
        let input = "{a: 1, b: 2}";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Map(vec![
                (Expr::Ident("a".to_string()), Expr::Literal(Literal::Int(1))),
                (Expr::Ident("b".to_string()), Expr::Literal(Literal::Int(2))),
            ])
        );
    }

    #[test]
    fn test_record_literal() {
        let input = "{ x = 1, y = 2 }";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Record(vec![
                ("x".to_string(), Expr::Literal(Literal::Int(1))),
                ("y".to_string(), Expr::Literal(Literal::Int(2))),
            ])
        );
    }

    #[test]
    fn test_where_expr() {
        let input = "x + y where { x = 1, y = 2 }";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Where {
                expr: Box::new(Expr::Binary {
                    op: BinaryOp::Add,
                    left: Box::new(Expr::Ident("x".to_string())),
                    right: Box::new(Expr::Ident("y".to_string())),
                }),
                bindings: vec![
                    ("x".to_string(), Expr::Literal(Literal::Int(1))),
                    ("y".to_string(), Expr::Literal(Literal::Int(2))),
                ],
            }
        );
    }

    #[test]
    fn test_otherwise_expr() {
        let input = "1 / 0 otherwise -1";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Otherwise {
                primary: Box::new(Expr::Binary {
                    op: BinaryOp::Div,
                    left: Box::new(Expr::Literal(Literal::Int(1))),
                    right: Box::new(Expr::Literal(Literal::Int(0))),
                }),
                fallback: Box::new(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(Expr::Literal(Literal::Int(1))),
                }),
            }
        );
    }

    #[test]
    fn test_lambda_no_argument() {
        let input = "() => 42";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Lambda {
                params: vec![],
                body: Box::new(Expr::Literal(Literal::Int(42))),
            }
        );
    }

    #[test]
    fn test_empty_record_literal() {
        let input = "Record {}";
        let parsed = parse(input).unwrap();
        assert_eq!(parsed, Expr::Record(vec![]));
    }

    #[test]
    fn test_format_string_with_interpolation() {
        let input = "f\" Hello, {a + b} !\\n \"";
        let parsed = parse(input).expect("parse failed");
        assert_eq!(
            parsed,
            Expr::FormatStr(vec![
                FormatSegment::Text(" Hello, ".to_string()),
                FormatSegment::Expr(Box::new(Expr::Binary {
                    op: BinaryOp::Add,
                    left: Box::new(Expr::Ident("a".to_string())),
                    right: Box::new(Expr::Ident("b".to_string())),
                })),
                FormatSegment::Text(" !\\n ".to_string()),
            ])
        );
    }

    #[test]
    fn test_where_expr_with_bindings() {
        let input = "x + y where { x = 1, y = 2, }";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Where {
                expr: Box::new(Expr::Binary {
                    op: BinaryOp::Add,
                    left: Box::new(Expr::Ident("x".to_string())),
                    right: Box::new(Expr::Ident("y".to_string())),
                }),
                bindings: vec![
                    ("x".to_string(), Expr::Literal(Literal::Int(1))),
                    ("y".to_string(), Expr::Literal(Literal::Int(2))),
                ],
            }
        );
    }

    #[test]
    fn test_function_call() {
        let input = "foo(1, 2, 3)";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Call {
                callable: Box::new(Expr::Ident("foo".to_string())),
                args: vec![
                    Expr::Literal(Literal::Int(1)),
                    Expr::Literal(Literal::Int(2)),
                    Expr::Literal(Literal::Int(3)),
                ],
            }
        );
    }

    #[test]
    fn test_index_access() {
        let input = "arr[42]";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Index {
                value: Box::new(Expr::Ident("arr".to_string())),
                index: Box::new(Expr::Literal(Literal::Int(42))),
            }
        );
    }

    #[test]
    fn test_attr_access() {
        let input = "obj.field";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Field {
                value: Box::new(Expr::Ident("obj".to_string())),
                field: "field".to_string(),
            }
        );
    }

    #[test]
    fn test_string_literal() {
        let input = "\"Hello, world!\"";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Literal(Literal::Str("Hello, world!".to_string()))
        );
    }

    #[test]
    fn test_bytes_literal() {
        let input = "b\"Hello, bytes!\"";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Literal(Literal::Bytes(b"Hello, bytes!".to_vec()))
        );
    }

    #[test]
    fn test_single_quoted_string_literal() {
        let input = "'Hello, world!'";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Literal(Literal::Str("Hello, world!".to_string()))
        );
    }

    #[test]
    fn test_single_quoted_bytes_literal() {
        let input = "b'Hello, bytes!'";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Literal(Literal::Bytes(b"Hello, bytes!".to_vec()))
        );
    }
}
