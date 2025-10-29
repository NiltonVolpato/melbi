use alloc::string::ToString;
use bumpalo::Bump;
use lazy_static::lazy_static;
use pest::Parser;
use pest::iterators::Pair;
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest_derive::Parser;

use crate::parser::parsed_expr::TypeExpr;
use crate::parser::syntax::AnnotatedSource;
use crate::parser::{BinaryOp, Expr, Literal, ParsedExpr, UnaryOp, syntax::Span};
use crate::{Vec, format};
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

struct ParseContext<'a, 'input> {
    arena: &'a Bump,
    original_source: &'input str, // To "transfer" slices to the arena allocated string.
    ann: &'a AnnotatedSource<'a, Expr<'a>>,
    depth: core::cell::Cell<usize>,
    max_depth: usize,
    // source: &'a str,
    // spans: RefCell<HashMap<*const Expr<'a>, Span, DefaultHashBuilder, &'a Bump>>,
}

impl<'a, 'input> ParseContext<'a, 'input> {
    // Returns a slice into `self.source` covering the same byte range that `s`
    // occupies within `self.original_source`.
    fn reslice(&self, s: &str) -> &'a str {
        let start = s.as_ptr() as usize - self.original_source.as_ptr() as usize;
        let end = start + s.len();
        &self.ann.source[start..end]
    }

    fn check_depth(&self, pair: &Pair<Rule>) -> Result<(), pest::error::Error<Rule>> {
        let current_depth = self.depth.get();
        if current_depth >= self.max_depth {
            return Err(pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: format!(
                        "Expression nesting depth exceeds maximum of {} levels. \
                         This likely indicates excessively nested parentheses or other constructs.",
                        self.max_depth
                    ),
                },
                pair.as_span(),
            ));
        }
        self.depth.set(current_depth + 1);
        Ok(())
    }

    fn parse_expr(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        self.check_depth(&pair)?;
        let result = match pair.as_rule() {
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
        };
        self.depth.set(self.depth.get() - 1);
        result
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
                let span =
                    Span::combine(&op.as_span().into(), &self.ann.span_of(rhs_value).unwrap());
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
                let span = Span::combine(
                    &self.ann.span_of(lhs_expr).unwrap(),
                    &self.ann.span_of(rhs_expr).unwrap(),
                );
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
                let span =
                    Span::combine(&self.ann.span_of(lhs_expr).unwrap(), &op.as_span().into());
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
        self.ann.add_span(node, span);
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
                then_branch: then_branch,
                else_branch: else_branch,
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

        let params: &'a [&'a str] = if let Some(params_pair) = pairs.next() {
            debug_assert_eq!(params_pair.as_rule(), Rule::lambda_params);
            let params = params_pair.into_inner().map(|p| self.reslice(p.as_str()));
            self.arena.alloc_slice_fill_iter(params)
        } else {
            &[]
        };

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
        let args = op.into_inner().map(|p| self.parse_expr(p));
        Ok(self.alloc_with_span(
            Expr::Call {
                callable,
                args: self.arena.alloc_slice_try_fill_iter(args)?,
            },
            span,
        ))
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
            .as_str();
        Ok(self.alloc_with_span(
            Expr::Field {
                value,
                field: self.reslice(field),
            },
            span,
        ))
    }

    fn parse_type_expr(&self, pair: Pair<Rule>) -> Result<TypeExpr<'a>, pest::error::Error<Rule>> {
        let pair_span = pair.as_span();
        match pair.as_rule() {
            Rule::type_expr => {
                // type_expr has one child: either record_type or (type_path with optional type_params)
                let mut inner = pair.into_inner();
                let first = inner.next().ok_or_else(|| {
                    pest::error::Error::new_from_span(
                        pest::error::ErrorVariant::CustomError {
                            message: "empty type expression".to_string(),
                        },
                        pair_span,
                    )
                })?;

                match first.as_rule() {
                    Rule::record_type => {
                        // Record[field1: Type1, field2: Type2, ...]
                        let fields_iter = first
                            .into_inner()
                            .map(|field_pair| self.parse_type_field(field_pair));
                        let fields = self.arena.alloc_slice_try_fill_iter(fields_iter)?;
                        Ok(TypeExpr::Record(fields))
                    }
                    Rule::type_path => {
                        let path = self.reslice(first.as_str());
                        // Check if there are type parameters (since type_params is silent, they appear as direct children)
                        let params_iter = inner.map(|p| self.parse_type_expr(p));
                        let params = self.arena.alloc_slice_try_fill_iter(params_iter)?;

                        if params.is_empty() {
                            Ok(TypeExpr::Path(path))
                        } else {
                            Ok(TypeExpr::Parametrized { path, params })
                        }
                    }
                    _ => Err(pest::error::Error::new_from_span(
                        pest::error::ErrorVariant::CustomError {
                            message: format!("unexpected rule in type_expr: {:?}", first.as_rule()),
                        },
                        first.as_span(),
                    )),
                }
            }
            _ => Err(pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: format!("expected type_expr, got {:?}", pair.as_rule()),
                },
                pair_span,
            )),
        }
    }

    fn parse_type_field(
        &self,
        pair: Pair<Rule>,
    ) -> Result<(&'a str, TypeExpr<'a>), pest::error::Error<Rule>> {
        let pair_span = pair.as_span();
        let mut inner = pair.into_inner();

        let ident = inner.next().ok_or_else(|| {
            pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: "missing field name".to_string(),
                },
                pair_span,
            )
        })?;
        let field_name = self.reslice(ident.as_str());

        let type_expr_pair = inner.next().ok_or_else(|| {
            pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: "missing field type".to_string(),
                },
                pair_span,
            )
        })?;
        let field_type = self.parse_type_expr(type_expr_pair)?;

        Ok((field_name, field_type))
    }

    fn parse_cast_expr(
        &self,
        expr: &'a Expr<'a>,
        op: Pair<Rule>,
        span: Span,
    ) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let op_span = op.as_span();
        let type_expr_pair = op.into_inner().next().ok_or_else(|| {
            pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: "missing type expression".to_string(),
                },
                op_span,
            )
        })?;
        let ty = self.parse_type_expr(type_expr_pair)?;
        Ok(self.alloc_with_span(Expr::Cast { ty, expr }, span))
    }

    fn parse_where_expr(
        &self,
        expr: &'a Expr<'a>,
        op: Pair<Rule>,
        span: Span,
    ) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let bindings_iter = op.into_inner().map(|p| self.parse_binding(p));
        let bindings = self.arena.alloc_slice_try_fill_iter(bindings_iter)?;
        Ok(self.alloc_with_span(Expr::Where { expr, bindings }, span))
    }

    fn parse_array(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let pair_span = pair.as_span();
        let items_iter = pair.into_inner().map(|p| self.parse_expr(p));
        let items = self.arena.alloc_slice_try_fill_iter(items_iter)?;
        let node = self.arena.alloc(Expr::Array(items));
        self.ann.add_span(node, pair_span.into());
        Ok(node)
    }

    fn parse_integer(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let pair_span = pair.as_span();
        let mut inner = pair.into_inner();
        let integer_number = inner.next().unwrap();

        // The integer_number is ${ "-"? ~ integer_literal }
        // So we can get the full signed string
        let number_str = integer_number.as_str().replace('_', "");

        // And check what kind of integer it is from the inner tokens
        let mut inner_num = integer_number.into_inner();
        let integer_type = inner_num.next().unwrap();

        let value = match integer_type.as_rule() {
            Rule::dec_integer => i64::from_str_radix(&number_str, 10),
            Rule::bin_integer => i64::from_str_radix(&number_str.replacen("0b", "", 1), 2),
            Rule::oct_integer => i64::from_str_radix(&number_str.replacen("0o", "", 1), 8),
            Rule::hex_integer => i64::from_str_radix(&number_str.replacen("0x", "", 1), 16),
            _ => unreachable!("Unknown integer format: {:?}", integer_type.as_rule()),
        }
        .map_err(|_| {
            pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: "invalid integer literal".to_string(),
                },
                pair_span,
            )
        })?;

        let suffix = match inner.next() {
            Some(s) => {
                debug_assert_eq!(s.as_rule(), Rule::suffix);
                Some(self.parse_expr(s.into_inner().next().unwrap())?)
            }
            None => None,
        };

        let node = self
            .arena
            .alloc(Expr::Literal(Literal::Int { value, suffix }));
        self.ann.add_span(node, pair_span.into());
        Ok(node)
    }

    fn parse_float(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let pair_span = pair.as_span();
        let mut inner = pair.into_inner();
        let float_number = inner.next().unwrap();

        // The float_number is ${ "-"? ~ float_literal }, so we can get the full signed string
        let value: f64 = float_number
            .as_str()
            .replace('_', "")
            .parse()
            .map_err(|_| {
                pest::error::Error::new_from_span(
                    pest::error::ErrorVariant::CustomError {
                        message: "invalid float literal".to_string(),
                    },
                    pair_span,
                )
            })?;

        let suffix = match inner.next() {
            Some(s) => {
                debug_assert_eq!(s.as_rule(), Rule::suffix);
                Some(self.parse_expr(s.into_inner().next().unwrap())?)
            }
            None => None,
        };

        let span = Span::from(pair_span);
        let node = self
            .arena
            .alloc(Expr::Literal(Literal::Float { value, suffix }));
        self.ann.add_span(node, span);
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
        let span = Span::from(pair_span);
        let node = self.arena.alloc(Expr::Literal(Literal::Bool(value)));
        self.ann.add_span(node, span);
        Ok(node)
    }

    fn parse_string(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let pair_span = pair.as_span();
        let s = pair.as_str();
        let inner = &s[1..s.len() - 1]; // Remove opening and closing quotes
        let inner_arena = self.reslice(inner); // Transfer to arena lifetime

        // Unescape the string literal
        let unescaped =
            crate::syntax::string_literal::unescape_string(self.arena, inner_arena, false)
                .map_err(|e| {
                    pest::error::Error::new_from_span(
                        pest::error::ErrorVariant::CustomError {
                            message: format!("Invalid string literal: {}", e),
                        },
                        pair_span,
                    )
                })?;

        let span = Span::from(pair_span);
        let node = self.arena.alloc(Expr::Literal(Literal::Str(unescaped)));
        self.ann.add_span(node, span);
        Ok(node)
    }

    fn parse_bytes(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let pair_span = pair.as_span();
        let s = pair.as_str();
        let inner = &s[2..s.len() - 1]; // Remove b" or b' prefix and closing quote
        let inner_arena = self.reslice(inner); // Transfer to arena lifetime

        // Unescape the bytes literal
        let bytes =
            crate::syntax::bytes_literal::unescape_bytes(self.arena, inner_arena).map_err(|e| {
                pest::error::Error::new_from_span(
                    pest::error::ErrorVariant::CustomError {
                        message: format!("Invalid bytes literal: {}", e),
                    },
                    pair_span,
                )
            })?;

        let span = Span::from(pair_span);
        let node = self.arena.alloc(Expr::Literal(Literal::Bytes(bytes)));
        self.ann.add_span(node, span);
        Ok(node)
    }

    fn parse_format_string(
        &self,
        pair: Pair<Rule>,
    ) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let pair_span = pair.as_span();
        let mut strs_vec = Vec::new();
        let mut exprs_vec = Vec::new();
        for segment in pair.into_inner() {
            match segment.as_rule() {
                Rule::format_text | Rule::format_text_single => {
                    // Unescape format string text (handles both {{ }} and escape sequences)
                    let text_arena = self.reslice(segment.as_str());
                    let unescaped = crate::syntax::string_literal::unescape_string(
                        self.arena, text_arena, true, // is_format_string
                    )
                    .map_err(|e| {
                        pest::error::Error::new_from_span(
                            pest::error::ErrorVariant::CustomError {
                                message: format!("Invalid format string: {}", e),
                            },
                            segment.as_span(),
                        )
                    })?;
                    strs_vec.push(unescaped);
                }
                Rule::format_expr => {
                    let expr = self.parse_expr(segment.into_inner().next().unwrap())?;
                    exprs_vec.push(expr);
                }
                _ => unreachable!("Unknown format string segment: {:?}", segment.as_rule()),
            }
        }
        let span = Span::from(pair_span);
        let node = self.arena.alloc(Expr::FormatStr {
            strs: self.arena.alloc_slice_copy(&strs_vec),
            exprs: self.arena.alloc_slice_copy(&exprs_vec),
        });
        self.ann.add_span(node, span);
        Ok(node)
    }

    fn parse_record(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let pair_span = pair.as_span();
        let fields_iter = pair.into_inner().map(|p| self.parse_binding(p));
        let fields = self.arena.alloc_slice_try_fill_iter(fields_iter)?;
        let span = Span::from(pair_span);
        let node = self.arena.alloc(Expr::Record(fields));
        self.ann.add_span(node, span);
        Ok(node)
    }

    fn parse_map(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let pair_span = pair.as_span();
        let entries_iter = pair.into_inner().map(|p| self.parse_map_entry(p));
        let entries = self.arena.alloc_slice_try_fill_iter(entries_iter)?;
        let span = Span::from(pair_span);
        let node = self.arena.alloc(Expr::Map(entries));
        self.ann.add_span(node, span);
        Ok(node)
    }

    fn parse_grouped(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        self.parse_expr(pair.into_inner().next().unwrap())
    }

    fn parse_ident(&self, pair: Pair<Rule>) -> Result<&'a Expr<'a>, pest::error::Error<Rule>> {
        let pair_span = pair.as_span();
        let span = Span::from(pair_span);
        let node = self.arena.alloc(Expr::Ident(self.reslice(pair.as_str())));
        self.ann.add_span(node, span);
        Ok(node)
    }

    fn parse_binding(
        &self,
        pair: Pair<Rule>,
    ) -> Result<(&'a str, &'a Expr<'a>), pest::error::Error<Rule>> {
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
            .as_str();
        let value = self.parse_expr(inner.next().ok_or_else(|| {
            pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: "missing binding value".to_string(),
                },
                span,
            )
        })?)?;
        Ok((self.reslice(name), value))
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

/// Default maximum nesting depth for expression parsing.
/// This prevents stack overflow from deeply nested expressions like `(((((...(1)...)))))`.
const DEFAULT_MAX_PARSE_DEPTH: usize = 500;

/// Parses a Melbi expression with the default maximum nesting depth.
///
/// For custom depth limits, use [`parse_with_max_depth`].
pub fn parse<'a, 'i>(
    arena: &'a Bump,
    source: &'i str,
) -> Result<&'a ParsedExpr<'a>, pest::error::Error<Rule>>
where
    'i: 'a,
{
    parse_with_max_depth(arena, source, DEFAULT_MAX_PARSE_DEPTH)
}

/// Parses a Melbi expression with a custom maximum nesting depth.
///
/// The `max_depth` parameter controls how deeply expressions can be nested
/// (e.g., parentheses, arrays, etc.) before returning an error. The default
/// limit used by [`parse`] is 1000.
///
/// This is useful for security-critical contexts where you want stricter limits,
/// or for testing/debugging where you need higher limits.
pub fn parse_with_max_depth<'a, 'i>(
    arena: &'a Bump,
    source: &'i str,
    max_depth: usize,
) -> Result<&'a ParsedExpr<'a>, pest::error::Error<Rule>>
where
    'i: 'a,
{
    let mut pairs = ExpressionParser::parse(Rule::main, source)?;
    let pair = pairs.next().unwrap(); // Safe: Rule::main always produces one pair.
    let context = ParseContext {
        arena,
        original_source: source, // To "transfer" slices to the arena allocated string.
        ann: arena.alloc(AnnotatedSource::new(arena, source)),
        depth: core::cell::Cell::new(0),
        max_depth,
        // source: arena.alloc_str(source),
        // spans: RefCell::new(HashMap::new_in(arena)),
    };
    let expr = context.parse_expr(pair)?;
    Ok(arena.alloc(ParsedExpr {
        expr,
        ann: context.ann,
        // spans: context.spans.into_inner(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_binary_expr() {
        let arena = Bump::new();
        let input = "1 + 2";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Binary {
                op: BinaryOp::Add,
                left: arena.alloc(Expr::Literal(Literal::Int {
                    value: 1,
                    suffix: None
                })),
                right: arena.alloc(Expr::Literal(Literal::Int {
                    value: 2,
                    suffix: None
                })),
            }
        );

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 5)));
        let Expr::Binary { left, right, .. } = parsed.expr else {
            panic!("Expected binary expression, got {:?}", parsed.expr);
        };
        assert_eq!(parsed.ann.span_of(left), Some(Span::new(0, 1)));
        assert_eq!(parsed.ann.span_of(right), Some(Span::new(4, 5)));
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

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 33)));
        let Expr::If {
            cond,
            then_branch,
            else_branch,
        } = parsed.expr
        else {
            panic!("Expected If expression");
        };
        assert_eq!(parsed.ann.span_of(cond), Some(Span::new(3, 12)));
        assert_eq!(parsed.ann.span_of(then_branch), Some(Span::new(18, 23)));
        assert_eq!(parsed.ann.span_of(else_branch), Some(Span::new(29, 33)));
    }

    #[test]
    fn test_lambda_expr() {
        let arena = Bump::new();
        let input = "(x) => x + 1";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Lambda {
                params: &["x"],
                body: arena.alloc(Expr::Binary {
                    op: BinaryOp::Add,
                    left: arena.alloc(Expr::Ident("x")),
                    right: arena.alloc(Expr::Literal(Literal::Int {
                        value: 1,
                        suffix: None
                    })),
                }),
            }
        );

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 12)));
        let Expr::Lambda { body, .. } = parsed.expr else {
            panic!("Expected Lambda expression");
        };
        assert_eq!(parsed.ann.span_of(body), Some(Span::new(7, 12)));
    }

    #[test]
    fn test_cast_expr_type_names() {
        let arena = Bump::new();
        let input = "m as Map[String, Integer]";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Cast {
                ty: TypeExpr::Parametrized {
                    path: "Map",
                    params: &[TypeExpr::Path("String"), TypeExpr::Path("Integer")]
                },
                expr: arena.alloc(Expr::Ident("m")),
            }
        );

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 25)));
    }

    #[test]
    fn test_array_literal() {
        let arena = Bump::new();
        let input = "[1, 2, 3]";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Array(&[
                arena.alloc(Expr::Literal(Literal::Int {
                    value: 1,
                    suffix: None
                })),
                arena.alloc(Expr::Literal(Literal::Int {
                    value: 2,
                    suffix: None
                })),
                arena.alloc(Expr::Literal(Literal::Int {
                    value: 3,
                    suffix: None
                })),
            ])
        );

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 9)));
        let Expr::Array(items) = parsed.expr else {
            panic!("Expected Array expression");
        };
        assert_eq!(parsed.ann.span_of(items[0]), Some(Span::new(1, 2)));
        assert_eq!(parsed.ann.span_of(items[1]), Some(Span::new(4, 5)));
        assert_eq!(parsed.ann.span_of(items[2]), Some(Span::new(7, 8)));
    }

    #[test]
    fn test_map_literal() {
        let arena = Bump::new();
        let input = "{a: 1, b: 2}";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Map(&[
                (
                    arena.alloc(Expr::Ident("a")),
                    arena.alloc(Expr::Literal(Literal::Int {
                        value: 1,
                        suffix: None
                    })),
                ),
                (
                    arena.alloc(Expr::Ident("b")),
                    arena.alloc(Expr::Literal(Literal::Int {
                        value: 2,
                        suffix: None
                    })),
                ),
            ])
        );

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 12)));
        let Expr::Map(entries) = parsed.expr else {
            panic!("Expected Map expression");
        };
        assert_eq!(parsed.ann.span_of(entries[0].0), Some(Span::new(1, 2)));
        assert_eq!(parsed.ann.span_of(entries[0].1), Some(Span::new(4, 5)));
        assert_eq!(parsed.ann.span_of(entries[1].0), Some(Span::new(7, 8)));
        assert_eq!(parsed.ann.span_of(entries[1].1), Some(Span::new(10, 11)));
    }

    #[test]
    fn test_record_literal() {
        let arena = Bump::new();
        let input = "{ x = 1, y = 2 }";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Record(&[
                (
                    "x",
                    arena.alloc(Expr::Literal(Literal::Int {
                        value: 1,
                        suffix: None
                    }))
                ),
                (
                    "y",
                    arena.alloc(Expr::Literal(Literal::Int {
                        value: 2,
                        suffix: None
                    }))
                ),
            ])
        );

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 16)));
        let Expr::Record(fields) = parsed.expr else {
            panic!("Expected Record expression");
        };
        assert_eq!(parsed.ann.span_of(fields[0].1), Some(Span::new(6, 7)));
        assert_eq!(parsed.ann.span_of(fields[1].1), Some(Span::new(13, 14)));
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
                    left: arena.alloc(Expr::Ident("x")),
                    right: arena.alloc(Expr::Ident("y")),
                }),
                bindings: &[
                    (
                        "x",
                        arena.alloc(Expr::Literal(Literal::Int {
                            value: 1,
                            suffix: None
                        }))
                    ),
                    (
                        "y",
                        arena.alloc(Expr::Literal(Literal::Int {
                            value: 2,
                            suffix: None
                        }))
                    ),
                ],
            }
        );

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 28)));
        let Expr::Where { expr, bindings } = parsed.expr else {
            panic!("Expected Where expression");
        };
        assert_eq!(parsed.ann.span_of(expr), Some(Span::new(0, 5)));
        assert_eq!(parsed.ann.span_of(bindings[0].1), Some(Span::new(18, 19)));
        assert_eq!(parsed.ann.span_of(bindings[1].1), Some(Span::new(25, 26)));
    }

    #[test]
    fn test_lambda_no_argument() {
        let arena = Bump::new();
        let input = "() => 42";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Lambda {
                params: &[],
                body: arena.alloc(Expr::Literal(Literal::Int {
                    value: 42,
                    suffix: None
                })),
            }
        );

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 8)));
        let Expr::Lambda { body, .. } = parsed.expr else {
            panic!("Expected Lambda expression");
        };
        assert_eq!(parsed.ann.span_of(body), Some(Span::new(6, 8)));
    }

    #[test]
    fn test_empty_record_literal() {
        let arena = Bump::new();
        let input = "Record {}";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(*parsed.expr, Expr::Record(&[]));

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 9)));
    }

    #[test]
    fn test_format_string_with_interpolation() {
        let arena = Bump::new();
        let input = "f\" Hello, {a + b} !\\n \"";
        let parsed = parse(&arena, input).expect("parse failed");

        assert_eq!(
            *parsed.expr,
            Expr::FormatStr {
                strs: &[" Hello, ", " !\n "],
                exprs: &[arena.alloc(Expr::Binary {
                    op: BinaryOp::Add,
                    left: arena.alloc(Expr::Ident("a")),
                    right: arena.alloc(Expr::Ident("b")),
                }),],
            }
        );

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 23)));
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
                    left: arena.alloc(Expr::Ident("x")),
                    right: arena.alloc(Expr::Ident("y")),
                }),
                bindings: &[
                    (
                        "x",
                        arena.alloc(Expr::Literal(Literal::Int {
                            value: 1,
                            suffix: None
                        }))
                    ),
                    (
                        "y",
                        arena.alloc(Expr::Literal(Literal::Int {
                            value: 2,
                            suffix: None
                        }))
                    ),
                ],
            }
        );

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 29)));
    }

    #[test]
    fn test_function_call() {
        let arena = Bump::new();
        let input = "foo(1, 2, 3)";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Call {
                callable: arena.alloc(Expr::Ident("foo")),
                args: &[
                    arena.alloc(Expr::Literal(Literal::Int {
                        value: 1,
                        suffix: None
                    })),
                    arena.alloc(Expr::Literal(Literal::Int {
                        value: 2,
                        suffix: None
                    })),
                    arena.alloc(Expr::Literal(Literal::Int {
                        value: 3,
                        suffix: None
                    })),
                ],
            }
        );

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 12)));
        let Expr::Call { callable, args } = parsed.expr else {
            panic!("Expected Call expression");
        };
        assert_eq!(parsed.ann.span_of(callable), Some(Span::new(0, 3)));
        assert_eq!(parsed.ann.span_of(args[0]), Some(Span::new(4, 5)));
        assert_eq!(parsed.ann.span_of(args[1]), Some(Span::new(7, 8)));
        assert_eq!(parsed.ann.span_of(args[2]), Some(Span::new(10, 11)));
    }

    #[test]
    fn test_index_access() {
        let arena = Bump::new();
        let input = "arr[42]";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Index {
                value: arena.alloc(Expr::Ident("arr")),
                index: arena.alloc(Expr::Literal(Literal::Int {
                    value: 42,
                    suffix: None
                })),
            }
        );

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 7)));
        let Expr::Index { value, index } = parsed.expr else {
            panic!("Expected Index expression");
        };
        assert_eq!(parsed.ann.span_of(value), Some(Span::new(0, 3)));
        assert_eq!(parsed.ann.span_of(index), Some(Span::new(4, 6)));
    }

    #[test]
    fn test_attr_access() {
        let arena = Bump::new();
        let input = "obj.field";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Field {
                value: &Expr::Ident("obj"),
                field: "field",
            }
        );

        println!("Parsed expr: {:#?}", parsed);
        println!("Span of expr: {:?}", parsed.ann.span_of(parsed.expr));
        println!("&expr: {:?}", parsed.expr as *const _);

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 9)));
        let Expr::Field { value, .. } = parsed.expr else {
            panic!("Expected Field expression");
        };
        assert_eq!(parsed.ann.span_of(*value), Some(Span::new(0, 3)));
    }

    #[test]
    fn test_string_literal() {
        let arena = Bump::new();
        let input = "\"Hello, world!\"";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("Hello, world!")));

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 15)));
    }

    #[test]
    fn test_bytes_literal() {
        let arena = Bump::new();
        let input = "b\"Hello, bytes!\"";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Bytes(b"Hello, bytes!"))
        );

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 16)));
    }

    #[test]
    fn test_bytes_escape_sequences() {
        let arena = Bump::new();

        // Newline
        let parsed = parse(&arena, r#"b"hello\nworld""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Bytes(b"hello\nworld")));

        // Tab, carriage return
        let parsed = parse(&arena, r#"b"a\tb\rc""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Bytes(b"a\tb\rc")));

        // Backslash, quotes
        let parsed = parse(&arena, r#"b"quote:\" slash:\\ end""#).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Bytes(b"quote:\" slash:\\ end"))
        );
    }

    #[test]
    fn test_bytes_hex_escapes() {
        let arena = Bump::new();

        // "Hello" in hex
        let parsed = parse(&arena, r#"b"\x48\x65\x6c\x6c\x6f""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Bytes(b"Hello")));

        // Mixed ASCII and hex
        let parsed = parse(&arena, r#"b"test\x20data""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Bytes(b"test data")));

        // Binary data (null, 0xFF, etc)
        let parsed = parse(&arena, r#"b"\x00\xff\x42""#).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Bytes(&[0x00, 0xff, 0x42]))
        );
    }

    #[test]
    fn test_bytes_quote_styles() {
        let arena = Bump::new();

        // Double quotes
        let parsed = parse(&arena, r#"b"hello""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Bytes(b"hello")));

        // Single quotes
        let parsed = parse(&arena, r#"b'hello'"#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Bytes(b"hello")));

        // Single quotes don't need escaping in double quotes
        let parsed = parse(&arena, r#"b"it's""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Bytes(b"it's")));

        // Double quotes don't need escaping in single quotes
        let parsed = parse(&arena, r#"b'say "hi"'"#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Bytes(b"say \"hi\"")));
    }

    #[test]
    fn test_bytes_null_escape() {
        let arena = Bump::new();

        let parsed = parse(&arena, r#"b"before\0after""#).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Bytes(b"before\0after"))
        );

        let parsed = parse(&arena, r#"b"\0\0\0""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Bytes(b"\0\0\0")));
    }

    #[test]
    fn test_bytes_line_continuation() {
        let arena = Bump::new();

        // Basic line continuation
        let parsed = parse(&arena, "b\"hello\\\nworld\"").unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Bytes(b"helloworld")));

        // Line continuation preserves following whitespace
        let parsed = parse(&arena, "b\"hello\\\n    world\"").unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Bytes(b"hello    world"))
        );

        // Multiple line continuations
        let parsed = parse(&arena, "b\"a\\\nb\\\nc\"").unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Bytes(b"abc")));
    }

    #[test]
    fn test_bytes_reject_non_ascii() {
        let arena = Bump::new();

        // Should fail with error about non-ASCII character 'é'
        let result = parse(&arena, r#"b"café""#);
        assert!(result.is_err());

        // Should fail with error about emoji
        let result = parse(&arena, r#"b"hello 🌍""#);
        assert!(result.is_err());
    }

    // ===== String literal unescaping tests =====

    #[test]
    fn test_string_escape_sequences() {
        let arena = Bump::new();
        let parsed = parse(&arena, r#""hello\nworld""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("hello\nworld")));

        let parsed = parse(&arena, r#""tab\there""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("tab\there")));

        let parsed = parse(&arena, r#""back\\slash""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("back\\slash")));
    }

    #[test]
    fn test_string_unicode_escapes() {
        let arena = Bump::new();
        // 4-digit Unicode escapes
        let parsed = parse(&arena, r#""\u0048\u0065\u006c\u006c\u006f""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("Hello")));

        let parsed = parse(&arena, r#""caf\u00e9""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("café")));

        // 8-digit Unicode escapes
        let parsed = parse(&arena, r#""\U0001F30D""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("🌍")));
    }

    #[test]
    fn test_string_quote_styles() {
        let arena = Bump::new();
        // Double quotes
        let parsed = parse(&arena, r#""hello""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("hello")));

        // Single quotes
        let parsed = parse(&arena, "'hello'").unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("hello")));

        // Escaped quotes
        let parsed = parse(&arena, r#""say \"hi\"""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str(r#"say "hi""#)));

        let parsed = parse(&arena, r"'say \'hi\''").unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("say 'hi'")));
    }

    #[test]
    fn test_string_utf8_in_source() {
        let arena = Bump::new();
        // UTF-8 characters should be allowed in source
        let parsed = parse(&arena, r#""café""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("café")));

        let parsed = parse(&arena, r#""🌍""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("🌍")));

        let parsed = parse(&arena, r#""hello世界""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("hello世界")));
    }

    #[test]
    fn test_string_null_escape() {
        let arena = Bump::new();
        let parsed = parse(&arena, r#""null\0byte""#).unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("null\0byte")));
    }

    #[test]
    fn test_string_line_continuation() {
        let arena = Bump::new();
        // Backslash + newline should be removed
        let parsed = parse(&arena, "\"hello\\\nworld\"").unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("helloworld")));

        // Following whitespace should be preserved
        let parsed = parse(&arena, "\"hello\\\n  world\"").unwrap();
        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("hello  world")));
    }

    // ===== Format string unescaping tests =====

    #[test]
    fn test_format_string_escape_sequences() {
        let arena = Bump::new();
        let parsed = parse(&arena, r#"f"hello\nworld""#).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::FormatStr {
                strs: &["hello\nworld"],
                exprs: &[],
            }
        );

        let parsed = parse(&arena, r#"f"tab\there""#).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::FormatStr {
                strs: &["tab\there"],
                exprs: &[],
            }
        );
    }

    #[test]
    fn test_format_string_single_quotes_escape_sequences() {
        let arena = Bump::new();
        let parsed = parse(&arena, r#"f'hello\nworld'"#).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::FormatStr {
                strs: &["hello\nworld"],
                exprs: &[],
            }
        );

        let parsed = parse(&arena, r#"f'tab\there'"#).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::FormatStr {
                strs: &["tab\there"],
                exprs: &[],
            }
        );
    }

    #[test]
    fn test_format_string_unicode_escapes() {
        let arena = Bump::new();
        let parsed = parse(&arena, r#"f"\u0048ello""#).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::FormatStr {
                strs: &["Hello"],
                exprs: &[],
            }
        );

        let parsed = parse(&arena, r#"f"\U0001F30D planet""#).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::FormatStr {
                strs: &["🌍 planet"],
                exprs: &[],
            }
        );
    }

    #[test]
    fn test_format_string_brace_and_escapes() {
        let arena = Bump::new();
        // Combine brace escaping and string escapes
        let parsed = parse(&arena, r#"f"{{\n}}""#).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::FormatStr {
                strs: &["{\n}"],
                exprs: &[],
            }
        );

        let parsed = parse(&arena, r#"f"Line 1\nLine 2\t{{literal}}""#).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::FormatStr {
                strs: &["Line 1\nLine 2\t{literal}"],
                exprs: &[],
            }
        );
    }

    #[test]
    fn test_format_string_mixed() {
        let arena = Bump::new();
        // Test format string with both expressions and escapes
        let parsed = parse(&arena, r#"f"text {x} more\ntext {{literal}}""#).unwrap();
        assert_eq!(
            *parsed.expr,
            Expr::FormatStr {
                strs: &["text ", " more\ntext {literal}"],
                exprs: &[arena.alloc(Expr::Ident("x"))],
            }
        );
    }

    #[test]
    fn test_single_quoted_string_literal() {
        let arena = Bump::new();
        let input = "'Hello, world!'";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(*parsed.expr, Expr::Literal(Literal::Str("Hello, world!")));

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 15)));
    }

    #[test]
    fn test_single_quoted_bytes_literal() {
        let arena = Bump::new();
        let input = "b'Hello, bytes!'";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Literal(Literal::Bytes(b"Hello, bytes!"))
        );

        assert_eq!(parsed.ann.span_of(parsed.expr), Some(Span::new(0, 16)));
    }

    #[test]
    fn test_integer_overflow() {
        let arena = Bump::new();
        let expr = "9223372036854775808"; // i64::MAX + 1
        let result = parse(&arena, expr);
        assert!(result.is_err(), "Expected failure parsing '{}'", expr);
    }

    #[test]
    fn test_cast_simple_type() {
        let arena = Bump::new();
        let input = "42 as Int";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Cast {
                ty: TypeExpr::Path("Int"),
                expr: &Expr::Literal(Literal::Int {
                    value: 42,
                    suffix: None
                }),
            }
        );
    }

    #[test]
    fn test_cast_parametrized_type() {
        let arena = Bump::new();
        let input = "x as Array[Int]";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Cast {
                ty: TypeExpr::Parametrized {
                    path: "Array",
                    params: &[TypeExpr::Path("Int")],
                },
                expr: arena.alloc(Expr::Ident("x")),
            }
        );
    }

    #[test]
    fn test_cast_map_type() {
        let arena = Bump::new();
        let input = "m as Map[String, Int]";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Cast {
                ty: TypeExpr::Parametrized {
                    path: "Map",
                    params: &[TypeExpr::Path("String"), TypeExpr::Path("Int")],
                },
                expr: arena.alloc(Expr::Ident("m")),
            }
        );
    }

    #[test]
    fn test_cast_nested_parametrized_type() {
        let arena = Bump::new();
        let input = "x as Array[Array[Int]]";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Cast {
                ty: TypeExpr::Parametrized {
                    path: "Array",
                    params: &[TypeExpr::Parametrized {
                        path: "Array",
                        params: &[TypeExpr::Path("Int")],
                    }],
                },
                expr: arena.alloc(Expr::Ident("x")),
            }
        );
    }

    #[test]
    fn test_cast_record_type() {
        let arena = Bump::new();
        let input = "r as Record[name: String, age: Int]";
        let parsed = parse(&arena, input).unwrap();

        assert_eq!(
            *parsed.expr,
            Expr::Cast {
                ty: TypeExpr::Record(&[
                    ("name", TypeExpr::Path("String")),
                    ("age", TypeExpr::Path("Int")),
                ]),
                expr: arena.alloc(Expr::Ident("r")),
            }
        );
    }

    #[test]
    fn test_depth_tracking_shallow() {
        // Test that shallow nesting works fine
        let arena = Bump::new();
        let input = "((((((((((1))))))))))"; // 10 levels of nesting
        let parsed = parse(&arena, input);
        assert!(parsed.is_ok(), "Shallow nesting should succeed");
    }

    #[test]
    fn test_depth_tracking_exceeds_limit() {
        // Test that exceeding max_depth fails with appropriate error
        let arena = Bump::new();
        let max_depth = 50;
        // Create expression with nesting that exceeds the limit
        let mut input = String::new();
        for _ in 0..max_depth + 10 {
            input.push('(');
        }
        input.push('1');
        for _ in 0..max_depth + 10 {
            input.push(')');
        }

        let parsed = parse_with_max_depth(&arena, &input, max_depth);
        assert!(parsed.is_err(), "Parsing beyond max_depth should fail");

        let err = parsed.unwrap_err();
        let err_msg = format!("{}", err);
        assert!(
            err_msg.contains("nesting depth exceeds maximum"),
            "Error message should mention nesting depth, got: {}",
            err_msg
        );
        assert!(
            err_msg.contains(&max_depth.to_string()),
            "Error message should mention the max depth limit"
        );
    }

    #[test]
    fn test_depth_tracking_arrays() {
        // Test depth tracking works with arrays too
        let arena = Bump::new();
        let max_depth = 30;
        // Create deeply nested array expressions
        let mut input = String::new();
        for _ in 0..max_depth + 5 {
            input.push('[');
        }
        input.push('1');
        for _ in 0..max_depth + 5 {
            input.push(']');
        }

        let parsed = parse_with_max_depth(&arena, &input, max_depth);
        assert!(parsed.is_err(), "Deeply nested arrays should fail");
    }

    #[test]
    fn test_depth_tracking_default_max() {
        // Verify the default max depth is reasonable (1000)
        let arena = Bump::new();
        // Create expression with 100 levels of nesting (well under default)
        let mut input = String::new();
        for _ in 0..100 {
            input.push('(');
        }
        input.push('1');
        for _ in 0..100 {
            input.push(')');
        }

        let parsed = parse(&arena, &input);
        assert!(parsed.is_ok(), "100 levels should be fine with default max");

        // Verify the default constant has the expected value
        assert_eq!(
            DEFAULT_MAX_PARSE_DEPTH, 500,
            "Default max depth should be 500"
        );
    }
}
