use bumpalo::Bump;
use hashbrown::{DefaultHashBuilder, HashMap};

use crate::parser::{
    BinaryOp, UnaryOp,
    syntax::{AnnotatedSource, Span},
};

#[derive(Debug)]
pub struct ParsedExpr<'a> {
    pub expr: &'a Expr<'a>,
    pub ann: AnnotatedSource<'a>,
    // pub source: &'a str,
    // pub spans: HashMap<*const Expr<'a>, Span, DefaultHashBuilder, &'a Bump>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'a> {
    Binary {
        op: BinaryOp,
        left: &'a Expr<'a>,
        right: &'a Expr<'a>,
    },
    Unary {
        op: UnaryOp,
        expr: &'a Expr<'a>,
    },
    Call {
        callable: &'a Expr<'a>,
        args: &'a [&'a Expr<'a>],
    },
    Index {
        value: &'a Expr<'a>,
        index: &'a Expr<'a>,
    },
    Field {
        value: &'a Expr<'a>,
        field: &'a str,
    },
    Cast {
        ty: TypeExpr<'a>,
        expr: &'a Expr<'a>,
    },
    Lambda {
        params: &'a [&'a str],
        body: &'a Expr<'a>,
    },
    If {
        cond: &'a Expr<'a>,
        then_branch: &'a Expr<'a>,
        else_branch: &'a Expr<'a>,
    },
    Where {
        expr: &'a Expr<'a>,
        bindings: &'a [(&'a str, &'a Expr<'a>)],
    },
    Otherwise {
        primary: &'a Expr<'a>,
        fallback: &'a Expr<'a>,
    },
    Record(&'a [(&'a str, &'a Expr<'a>)]),
    Map(&'a [(&'a Expr<'a>, &'a Expr<'a>)]),
    Array(&'a [&'a Expr<'a>]),
    FormatStr {
        // REQUIRES: strs.len() == exprs.len() + 1
        strs: &'a [&'a str],
        exprs: &'a [&'a Expr<'a>],
    },
    Literal(Literal<'a>),
    Ident(&'a str),
}

impl<'a> Expr<'a> {
    pub fn as_ptr(&self) -> *const Self {
        self as *const _
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal<'a> {
    Int {
        value: i64,
        suffix: Option<&'a Expr<'a>>,
    },
    Float {
        value: f64,
        suffix: Option<&'a Expr<'a>>,
    },
    Bool(bool),
    Str(&'a str),
    Bytes(&'a [u8]),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr<'a> {
    Path(&'a str),
    Parametrized {
        path: &'a str,
        params: &'a [TypeExpr<'a>],
    },
    Record(&'a [(&'a str, TypeExpr<'a>)]),
}
