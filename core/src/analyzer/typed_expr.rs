use crate::{
    parser::{BinaryOp, BoolOp, UnaryOp},
    types::Type,
    values::dynamic::Value,
};

#[derive(Debug)]
pub struct TypedExpr<'types, 'arena> {
    pub source: &'arena str,
    pub expr: &'arena Expr<'types, 'arena>,
    // spans: HashMap<*const Expr<'types, 'arena>, Span, DefaultHashBuilder, &'arena Bump>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr<'types, 'arena>(pub &'types Type<'types>, pub ExprInner<'types, 'arena>);

impl<'types, 'arena> Expr<'types, 'arena> {
    pub fn as_ptr(&self) -> *const Self {
        self as *const _
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprInner<'types, 'arena> {
    Binary {
        op: BinaryOp,
        left: &'arena Expr<'types, 'arena>,
        right: &'arena Expr<'types, 'arena>,
    },
    Boolean {
        op: BoolOp,
        left: &'arena Expr<'types, 'arena>,
        right: &'arena Expr<'types, 'arena>,
    },
    Unary {
        op: UnaryOp,
        expr: &'arena Expr<'types, 'arena>,
    },
    Call {
        callable: &'arena Expr<'types, 'arena>,
        args: &'arena [&'arena Expr<'types, 'arena>],
    },
    Index {
        value: &'arena Expr<'types, 'arena>,
        index: &'arena Expr<'types, 'arena>,
    },
    Field {
        value: &'arena Expr<'types, 'arena>,
        field: &'arena str,
    },
    Cast {
        expr: &'arena Expr<'types, 'arena>,
    },
    Lambda {
        params: &'arena [&'arena str],
        body: &'arena Expr<'types, 'arena>,
    },
    If {
        cond: &'arena Expr<'types, 'arena>,
        then_branch: &'arena Expr<'types, 'arena>,
        else_branch: &'arena Expr<'types, 'arena>,
    },
    Where {
        expr: &'arena Expr<'types, 'arena>,
        bindings: &'arena [(&'arena str, &'arena Expr<'types, 'arena>)],
    },
    Otherwise {
        primary: &'arena Expr<'types, 'arena>,
        fallback: &'arena Expr<'types, 'arena>,
    },
    Record {
        fields: &'arena [(&'arena str, &'arena Expr<'types, 'arena>)],
    },
    Map {
        elements: &'arena [(&'arena Expr<'types, 'arena>, &'arena Expr<'types, 'arena>)],
    },
    Array {
        elements: &'arena [&'arena Expr<'types, 'arena>],
    },
    FormatStr {
        // REQUIRES: strs.len() == exprs.len() + 1
        strs: &'arena [&'arena str],
        exprs: &'arena [&'arena Expr<'types, 'arena>],
    },
    Constant(Value<'types, 'arena>),
    Ident(&'arena str),
}
