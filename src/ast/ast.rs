#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Call {
        function: Box<Expr>,
        args: Vec<Expr>,
    },
    Index {
        collection: Box<Expr>, // object?
        index: Box<Expr>,
    },
    Attr {
        object: Box<Expr>,
        field: String,
    },
    Cast {
        expr: Box<Expr>,
        ty: TypeExpr,
    },
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Where {
        expr: Box<Expr>,
        bindings: Vec<(String, Expr)>,
    },
    Otherwise {
        primary: Box<Expr>,
        fallback: Box<Expr>,
    },
    Record(Vec<(String, Expr)>),
    Map(Vec<(Expr, Expr)>),
    Array(Vec<Expr>),
    Literal(Literal),
    Ident(String),
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
    FormatStr(Vec<FormatPart>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormatPart {
    Text(String),    // Represents plain text within the format string
    Expr(Box<Expr>), // Represents embedded expressions
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Path(String),
    Parametrized { path: String, params: Vec<TypeExpr> },
    Record(Vec<(String, TypeExpr)>),
}
