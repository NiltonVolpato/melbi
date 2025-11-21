use std::fmt::Debug;

pub trait Context: Sized + Clone {
    type ExprHandle: Clone + Debug + PartialEq + Eq;

    fn alloc_expr(&mut self, data: Span, kind: ExprKind<Self>) -> Self::ExprHandle;

    fn resolve_expr<'a>(&'a self, handle: &'a Self::ExprHandle) -> &'a ExprKind<Self>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Span(pub usize, pub usize);

#[derive(Clone, PartialEq, Eq)]
pub enum ExprKind<C: Context> {
    Lit(i32),
    Add(C::ExprHandle, C::ExprHandle),
}

// Manual Debug to keep output clean
impl<C: Context> Debug for ExprKind<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lit(x) => write!(f, "Lit({})", x),
            Self::Add(l, r) => f.debug_tuple("Add").field(l).field(r).finish(),
        }
    }
}

// 4. Folder (Transformer)
pub struct Folder<'a, In: Context, Out: Context> {
    pub source: &'a In,
    pub target: &'a mut Out,
}

impl<'a, In: Context, Out: Context> Folder<'a, In, Out> {
    pub fn new(source: &'a In, target: &'a mut Out) -> Self {
        Self { source, target }
    }

    pub fn fold_expr(&mut self, handle: &In::ExprHandle) -> Out::ExprHandle {
        let kind_in = self.source.resolve_expr(handle);
        let kind_out = match kind_in {
            ExprKind::Lit(i) => ExprKind::Lit(*i),
            ExprKind::Add(l, r) => ExprKind::Add(self.fold_expr(l), self.fold_expr(r)),
        };
        // Note: We use default span here for simplicity
        self.target.alloc_expr(Span::default(), kind_out)
    }
}
