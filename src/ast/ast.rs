use std::collections::HashMap;

pub struct ParsedExpr<'a> {
    pub source: String,
    pub root: &'a Expr<'a>, // XXX: rename to expr.
    pub spans: HashMap<*const Expr<'a>, Span>,
}

impl<'a> ParsedExpr<'a> {
    pub fn span_of(&'a self, expr: &Expr<'a>) -> Option<Span> {
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
        args: Vec<&'a Expr<'a>>, // XXX
    },
    Index {
        value: &'a Expr<'a>,
        index: &'a Expr<'a>,
    },
    Field {
        value: &'a Expr<'a>,
        field: String,
    },
    Cast {
        expr: &'a Expr<'a>,
        ty: TypeExpr,
    },
    Lambda {
        params: Vec<String>,
        body: &'a Expr<'a>,
    },
    If {
        cond: &'a Expr<'a>,
        then_branch: &'a Expr<'a>,
        else_branch: &'a Expr<'a>,
    },
    Where {
        expr: &'a Expr<'a>,
        bindings: Vec<(String, &'a Expr<'a>)>, // XXX
    },
    Otherwise {
        primary: &'a Expr<'a>,
        fallback: &'a Expr<'a>,
    },
    Record(Vec<(String, &'a Expr<'a>)>),    // XXX
    Map(Vec<(&'a Expr<'a>, &'a Expr<'a>)>), // XXX
    Array(Vec<&'a Expr<'a>>),               // XXX
    FormatStr(Vec<FormatSegment<'a>>),
    Literal(Literal),
    Ident(String),
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
pub enum Literal {
    Int(i64),
    Float(f64),
    Str(String),
    Bytes(Vec<u8>),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormatSegment<'a> {
    Text(String),       // Represents plain text within the format string
    Expr(&'a Expr<'a>), // Represents embedded expressions
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Path(String),
    Parametrized { path: String, params: Vec<TypeExpr> },
    Record(Vec<(String, TypeExpr)>),
}
