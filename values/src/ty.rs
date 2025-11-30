// ty.rs

use alloc::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TyKind {
    Int,
    Bool,
    Array(Rc<TyKind>),
}

impl TyKind {
    pub fn handle(self) -> Rc<Self> {
        Rc::new(self)
    }
}
