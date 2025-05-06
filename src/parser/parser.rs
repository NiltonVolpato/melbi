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
        .op(Op::postfix(Rule::where_op))                 // `where {}`  // XXX

        .op(Op::infix(Rule::otherwise_op, Assoc::Right)) // `otherwise` // XXX

        .op(Op::prefix(Rule::if_op))                     // `if`  // XXX

        // Boolean operators.
        .op(Op::infix(Rule::or, Assoc::Left))            // `or`
        .op(Op::infix(Rule::and, Assoc::Left))           // `and`
        .op(Op::prefix(Rule::not))                       // `not`  // XXX

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

        .op(Op::postfix(Rule::call_op))                  // `()`
        .op(Op::postfix(Rule::index_op))                 // `[]`
        .op(Op::postfix(Rule::field_op))                  // `.`  // XXX: add more precedence tests
        .op(Op::postfix(Rule::cast_op))                  // `as`
        // (highest precedence)
        ;
}

#[derive(Parser)]
#[grammar = "parser/expression.pest"]
pub struct ExpressionParser;

pub fn parse_expr(pair: Pair<Rule>) -> Expr {
    match pair.as_rule() {
        Rule::main => parse_expr(
            pair.into_inner()
                .next()
                .expect("missing expected pair in rule"),
        ),

        Rule::expression => PRATT_PARSER
            .map_primary(|primary| parse_expr(primary))
            .map_prefix(|op, rhs| {
                let op = match op.as_rule() {
                    Rule::neg => UnaryOp::Neg,
                    Rule::not => UnaryOp::Not,
                    Rule::if_op => {
                        let mut pairs = op.into_inner();
                        let cond = parse_expr(pairs.next().unwrap());
                        let then_branch = parse_expr(pairs.next().unwrap());
                        return Expr::If {
                            cond: Box::new(cond),
                            then_branch: Box::new(then_branch),
                            else_branch: Box::new(rhs),
                        };
                    }
                    _ => unreachable!("Unknown unary operator: {:?}", op.as_rule()),
                };
                Expr::Unary {
                    op,
                    expr: Box::new(rhs),
                }
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
                        return Expr::Otherwise {
                            primary: Box::new(lhs),
                            fallback: Box::new(rhs),
                        };
                    }
                    _ => unreachable!("Unknown binary operator: {:?}", op.as_rule()),
                };
                Expr::Binary {
                    op,
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                }
            })
            .map_postfix(|lhs, op| match op.as_rule() {
                Rule::call_op => {
                    let args = op.into_inner().map(parse_expr).collect();
                    Expr::Call {
                        callable: Box::new(lhs),
                        args,
                    }
                }
                Rule::index_op => {
                    let index_expr =
                        parse_expr(op.into_inner().next().expect("missing index expression"));
                    Expr::Index {
                        value: Box::new(lhs),
                        index: Box::new(index_expr),
                    }
                }
                Rule::field_op => {
                    let field = op
                        .into_inner()
                        .next()
                        .expect("missing attribute ident")
                        .as_str()
                        .to_string();
                    Expr::Field {
                        value: Box::new(lhs),
                        field,
                    }
                }
                Rule::cast_op => {
                    // TODO: Implement type parsing.
                    let ty = crate::ast::TypeExpr::Path(
                        op.into_inner()
                            .next()
                            .expect("missing type expression")
                            .as_str()
                            .trim()
                            .to_string(),
                    );
                    Expr::Cast {
                        expr: Box::new(lhs),
                        ty,
                    }
                }
                Rule::where_op => {
                    let bindings = op.into_inner().map(parse_binding).collect();
                    Expr::Where {
                        expr: Box::new(lhs), // Wrap the entire preceding expression
                        bindings,
                    }
                }
                _ => unreachable!("Unknown postfix operator: {:?}", op.as_rule()),
            })
            .parse(pair.into_inner()),

        // Rule::if_expr => {
        //     let mut pairs = pair.into_inner();
        //     let cond = parse_expr(pairs.next().unwrap());
        //     let then_branch = parse_expr(pairs.next().unwrap());
        //     let else_branch = parse_expr(pairs.next().unwrap());
        //     Expr::If {
        //         cond: Box::new(cond),
        //         then_branch: Box::new(then_branch),
        //         else_branch: Box::new(else_branch),
        //     }
        // }
        Rule::array => {
            let items = pair.into_inner().map(parse_expr).collect();
            Expr::Array(items)
        }

        Rule::lambda => {
            let mut pairs = pair.into_inner();
            let params_pair = pairs.next().unwrap();
            let mut param_idents = Vec::new();
            if params_pair.as_rule() == Rule::lambda_params {
                param_idents = params_pair
                    .into_inner()
                    .map(|p| p.as_str().to_string())
                    .collect();
                let body = parse_expr(pairs.next().unwrap());
                Expr::Lambda {
                    params: param_idents,
                    body: Box::new(body),
                }
            } else {
                let body = parse_expr(params_pair);
                Expr::Lambda {
                    params: param_idents,
                    body: Box::new(body),
                }
            }
        }

        Rule::integer => {
            let value = pair.as_str().parse().unwrap();
            Expr::Literal(Literal::Int(value))
        }

        Rule::float => {
            let value = pair.as_str().parse().unwrap();
            Expr::Literal(Literal::Float(value))
        }

        Rule::boolean => {
            let value = match pair.as_str() {
                "true" => true,
                "false" => false,
                _ => unreachable!(),
            };
            Expr::Literal(Literal::Bool(value))
        }

        Rule::string => {
            let s = pair.as_str();
            let inner = &s[1..s.len() - 1];
            Expr::Literal(Literal::Str(inner.to_string()))
        }

        Rule::bytes => {
            let s = pair.as_str();
            let inner = &s[2..s.len() - 1];
            Expr::Literal(Literal::Bytes(inner.as_bytes().to_vec()))
        }

        Rule::format_string => {
            let mut parts = Vec::new();
            for inner in pair.into_inner() {
                match inner.as_rule() {
                    Rule::format_text => {
                        parts.push(FormatSegment::Text(inner.as_str().to_string()));
                    }
                    Rule::format_expr => {
                        let expr = parse_expr(inner.into_inner().next().unwrap());
                        parts.push(FormatSegment::Expr(Box::new(expr)));
                    }
                    _ => unreachable!("Unexpected rule in format_string: {:?}", inner.as_rule()),
                }
            }
            Expr::FormatStr(parts)
        }

        Rule::record => {
            let fields = pair
                .into_inner()
                .map(|field| {
                    let mut inner = field.into_inner();
                    let name = inner.next().unwrap().as_str().to_string();
                    let value = parse_expr(inner.next().unwrap());
                    (name, value)
                })
                .collect();
            Expr::Record(fields)
        }

        Rule::map => {
            let entries = pair.into_inner().map(parse_map_entry).collect();
            Expr::Map(entries)
        }

        Rule::grouped => parse_expr(pair.into_inner().next().unwrap()),

        Rule::ident => Expr::Ident(pair.as_str().to_string()),

        _ => unimplemented!("Unhandled rule: {:?}", pair.as_rule()),
    }
}

fn parse_binding(pair: Pair<Rule>) -> (String, Expr) {
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .expect("missing binding name")
        .as_str()
        .to_string();
    let value = parse_expr(inner.next().expect("missing binding value"));
    (name, value)
}

fn parse_map_entry(pair: Pair<Rule>) -> (Expr, Expr) {
    let mut inner = pair.into_inner();
    let key = parse_expr(inner.next().expect("missing map key"));
    let value = parse_expr(inner.next().expect("missing map value"));
    (key, value)
}

pub fn parse(source: &str) -> Result<Expr, pest::error::Error<Rule>> {
    let mut pairs = ExpressionParser::parse(Rule::main, source)?;
    let pair = pairs.next().unwrap();
    Ok(parse_expr(pair))
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
