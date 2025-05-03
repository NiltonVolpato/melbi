use crate::ast::{BinaryOp, Expr, Literal, UnaryOp};
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "parser/expression.pest"]
pub struct ExpressionParser;

pub fn parse_expr(pair: Pair<Rule>) -> Expr {
    match pair.as_rule() {
        Rule::main => parse_expr(pair.into_inner().next().unwrap()),

        Rule::expression => parse_expr(pair.into_inner().next().unwrap()),

        Rule::if_expr => {
            let mut pairs = pair.into_inner();
            let cond = parse_expr(pairs.next().unwrap());
            let then_branch = parse_expr(pairs.next().unwrap());
            let else_branch = parse_expr(pairs.next().unwrap());
            Expr::If {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            }
        }

        Rule::otherwise_expr => {
            let mut pairs = pair.into_inner();
            let primary = parse_expr(pairs.next().unwrap());
            if let Some(fallback_pair) = pairs.next() {
                let fallback = parse_expr(fallback_pair);
                Expr::Otherwise {
                    primary: Box::new(primary),
                    fallback: Box::new(fallback),
                }
            } else {
                primary
            }
        }

        Rule::where_expr => {
            let mut pairs = pair.into_inner();
            let expr = parse_expr(pairs.next().unwrap());
            let bindings = pairs
                .next()
                .unwrap()
                .into_inner()
                .map(|binding| {
                    let mut inner = binding.into_inner();
                    let name = inner.next().unwrap().as_str().to_string();
                    let value = parse_expr(inner.next().unwrap());
                    (name, value)
                })
                .collect();
            Expr::Where {
                expr: Box::new(expr),
                bindings,
            }
        }

        Rule::binary_expr => {
            let mut pairs = pair.into_inner();
            let first = parse_expr(pairs.next().unwrap());
            let mut expr = first;

            while let Some(op_pair) = pairs.next() {
                let op = match op_pair.as_str() {
                    "+" => BinaryOp::Add,
                    "-" => BinaryOp::Sub,
                    "*" => BinaryOp::Mul,
                    "/" => BinaryOp::Div,
                    "^" => BinaryOp::Pow,
                    "and" => BinaryOp::And,
                    "or" => BinaryOp::Or,
                    _ => unreachable!("Unknown binary operator: {}", op_pair.as_str()),
                };
                let right = parse_expr(pairs.next().unwrap());
                expr = Expr::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                };
            }

            expr
        }

        Rule::unary_expr => {
            let mut pairs = pair.into_inner();
            let mut ops = Vec::new();
            while let Some(p) = pairs.peek() {
                if p.as_rule() == Rule::unary_op {
                    let op = match p.as_str() {
                        "-" => UnaryOp::Neg,
                        "not" => UnaryOp::Not,
                        _ => unreachable!("Unknown unary operator: {}", p.as_str()),
                    };
                    ops.push(op);
                    pairs.next();
                } else {
                    break;
                }
            }
            let mut expr = parse_expr(pairs.next().unwrap());
            for op in ops.into_iter().rev() {
                expr = Expr::Unary {
                    op,
                    expr: Box::new(expr),
                };
            }
            expr
        }

        Rule::cast_expr => {
            let mut pairs = pair.into_inner();
            let expr = parse_expr(pairs.next().unwrap());
            if let Some(type_pair) = pairs.next() {
                let ty = crate::ast::TypeExpr::Path(type_pair.as_str().to_string());
                Expr::Cast {
                    expr: Box::new(expr),
                    ty,
                }
            } else {
                expr
            }
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
            let s = pair.as_str();
            Expr::Literal(Literal::FormatStr(vec![crate::ast::FormatPart::Text(
                s.to_string(),
            )]))
        }

        Rule::array => {
            let elements = pair.into_inner().map(parse_expr).collect();
            Expr::Array(elements)
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
            let entries = pair
                .into_inner()
                .map(|entry| {
                    let mut inner = entry.into_inner();
                    let key = parse_expr(inner.next().unwrap());
                    let value = parse_expr(inner.next().unwrap());
                    (key, value)
                })
                .collect();
            Expr::Map(entries)
        }

        Rule::grouped => {
            let inner = parse_expr(pair.into_inner().next().unwrap());
            Expr::Group(Box::new(inner))
        }

        Rule::ident => Expr::Ident(pair.as_str().to_string()),

        _ => unimplemented!("Unhandled rule: {:?}", pair.as_rule()),
    }
}

pub fn parse(source: &str) -> Result<Expr, pest::error::Error<Rule>> {
    let mut pairs = ExpressionParser::parse(Rule::main, source)?;
    let pair = pairs.next().unwrap();
    Ok(parse_expr(pair))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{FormatPart, TypeExpr};

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
        let input = "if true then 1 else 0";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::If {
                cond: Box::new(Expr::Literal(Literal::Bool(true))),
                then_branch: Box::new(Expr::Literal(Literal::Int(1))),
                else_branch: Box::new(Expr::Literal(Literal::Int(0))),
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
    fn test_grouped_expr() {
        let input = "(1 + 2)";
        let parsed = parse(input).unwrap();
        assert_eq!(
            parsed,
            Expr::Group(Box::new(Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expr::Literal(Literal::Int(1))),
                right: Box::new(Expr::Literal(Literal::Int(2))),
            }))
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
        let input = "Record { x = 1, y = 2 }";
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
        let input = "f\"Hello, {name}!\"";
        let parsed = parse(input).expect("parse failed");
        assert_eq!(
            parsed,
            Expr::Literal(Literal::FormatStr(vec![
                FormatPart::Text("Hello, ".to_string()),
                FormatPart::Expr(Expr::Ident("name".to_string())),
                FormatPart::Text("!".to_string()),
            ]))
        );
    }
}
