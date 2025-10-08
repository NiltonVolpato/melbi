use bumpalo::Bump;
use hashbrown::{DefaultHashBuilder, HashMap};

pub struct ParsedExpr<'a> {
    pub source: &'a str,
    pub expr: &'a Expr<'a>,
    pub spans: HashMap<*const Expr<'a>, Span, DefaultHashBuilder, &'a Bump>,
}

impl<'a> ParsedExpr<'a> {
    pub fn span_of(&self, expr: &Expr<'a>) -> Option<Span> {
        self.spans.get(&expr.as_ptr()).copied()
    }

    pub fn snippet(&self, span: Span) -> &str {
        &self.source[span.start..span.end]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
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
        expr: &'a Expr<'a>,
        ty: TypeExpr<'a>,
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
    FormatStr(&'a [FormatSegment<'a>]),
    Literal(Literal<'a>),
    Ident(&'a str),
}

impl<'a> Expr<'a> {
    pub fn as_ptr(&self) -> *const Self {
        self as *const _
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal<'a> {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(&'a str),
    Bytes(&'a [u8]),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormatSegment<'a> {
    Text(&'a str),      // Represents plain text within the format string
    Expr(&'a Expr<'a>), // Represents embedded expressions
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
