use pest::Parser;
use pest::iterators::Pair;
use crate::ast::{Expr, BinaryOp, UnaryOp, Literal};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "parser/expression.pest"]
pub struct ExpressionParser;


pub fn parse_expr(pair: Pair<Rule>) -> Expr {
    match pair.as_rule() {
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
        },

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
                // Stub for type parsing
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
            } else {
                // It's already the body; push it back
                let chained_pairs: Vec<_> = std::iter::once(params_pair).chain(pairs).collect();
                pairs = chained_pairs.into_iter();
            }
            let body = parse_expr(pairs.next().unwrap());
            Expr::Lambda {
                params: param_idents,
                body: Box::new(body),
            }
        }

        Rule::grouped => {
            let inner = parse_expr(pair.into_inner().next().unwrap());
            Expr::Group(Box::new(inner))
        },

        Rule::bytes => {
            let s = pair.as_str();
            let inner = &s[2..s.len() - 1];
            Expr::Literal(Literal::Bytes(inner.as_bytes().to_vec()))
        },

        Rule::format_string => {
            let parts = pair.into_inner().map(|p| match p.as_rule() {
                // Rule::string_part => Literal::FormatStr(vec![crate::ast::FormatPart::Text(p.as_str().to_string())]),
                // Rule::escape_seq => Literal::FormatStr(vec![crate::ast::FormatPart::Escape(p.as_str().to_string())]),
                Rule::expression => Literal::FormatStr(vec![crate::ast::FormatPart::Expr(Box::new(parse_expr(p)))]),
                _ => unreachable!(),
            }).flat_map(|f| match f {
                Literal::FormatStr(parts) => parts,
                _ => unreachable!(),
            }).collect();
            Expr::Literal(Literal::FormatStr(parts))
        },

        Rule::array => {
            let elements = pair.into_inner().map(parse_expr).collect();
            Expr::Array(elements)
        },

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
        },

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
        },

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
            // Note: real escape parsing should happen here
            let s = pair.as_str();
            let inner = &s[1..s.len()-1];
            Expr::Literal(Literal::Str(inner.to_string()))
        }

        Rule::ident => {
            Expr::Ident(pair.as_str().to_string())
        }

        _ => unimplemented!("Unhandled rule: {:?}", pair.as_rule()),
    }
}


pub fn parse_str_to_expr(source: &str) -> Result<Expr, pest::error::Error<Rule>> {
    let mut pairs = ExpressionParser::parse(Rule::main, source)?;
    let pair = pairs.next().unwrap();
    Ok(parse_expr(pair))
}
