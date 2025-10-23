// These are common syntax structures used in ParsedExpr and TypedExpr.

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn str_of<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    And,
    Or,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}
