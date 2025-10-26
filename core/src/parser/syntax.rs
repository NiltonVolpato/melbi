// These are common syntax structures used in ParsedExpr and TypedExpr.

use core::{cell::RefCell, ops::Range};

use bumpalo::Bump;
use hashbrown::{DefaultHashBuilder, HashMap};

#[derive(Debug)]
pub struct AnnotatedSource<'a, T> {
    pub source: &'a str,
    spans: RefCell<HashMap<*const T, Span, DefaultHashBuilder, &'a Bump>>,
}

impl<'a, T> AnnotatedSource<'a, T> {
    pub fn new(arena: &'a Bump, source: &'a str) -> Self {
        Self {
            source,
            spans: RefCell::new(HashMap::new_in(arena)),
        }
    }
    pub fn add_span(&self, expr: &T, span: Span) {
        let p = expr as *const _;
        self.spans.borrow_mut().insert(p, span);
    }
    pub fn span_of(&self, expr: &T) -> Option<Span> {
        let p = expr as *const _;
        self.spans.borrow().get(&p).cloned()
    }
    pub fn snippet(&self, span: Span) -> &str {
        &self.source[span.0]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span(pub Range<usize>);

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self(start..end)
    }
    pub fn combine(a: &Span, b: &Span) -> Span {
        Span::new(a.0.start, b.0.end)
    }
    pub fn str_of<'a>(&self, source: &'a str) -> &'a str {
        &source[self.0.start..self.0.end]
    }
}

impl From<pest::Span<'_>> for Span {
    fn from(s: pest::Span<'_>) -> Self {
        Self(s.start()..s.end())
    }
}

// impl Deref for Span {
//     type Target = Range<usize>;
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

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
