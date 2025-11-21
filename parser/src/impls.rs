use super::traits::{Context, ExprKind, Span};
use std::fmt::Debug;

// --- IMPL 1: Heap Context (Standard) ---

// 1. Define the Node
// FIX: Added PartialEq, Eq
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeapNode<C: Context> {
    pub data: Span,
    pub kind: ExprKind<C>,
}

// 2. Define the Handle Wrapper
// FIX: We wrap the Box in a struct to break the recursive type alias cycle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeapHandle(pub Box<HeapNode<HeapCtx>>);

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct HeapCtx;

impl Context for HeapCtx {
    type ExprHandle = HeapHandle;

    fn alloc_expr(&mut self, data: Span, kind: ExprKind<Self>) -> Self::ExprHandle {
        HeapHandle(Box::new(HeapNode { data, kind }))
    }

    fn resolve_expr<'a>(&'a self, handle: &'a Self::ExprHandle) -> &'a ExprKind<Self> {
        &handle.0.kind
    }
}

// --- IMPL 2: View Context (For Test Results) ---

// FIX: Explicit struct wrapper
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewHandle(pub Box<ExprKind<ViewCtx>>);

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct ViewCtx;

impl Context for ViewCtx {
    type ExprHandle = ViewHandle;

    fn alloc_expr(&mut self, _data: Span, kind: ExprKind<Self>) -> Self::ExprHandle {
        ViewHandle(Box::new(kind))
    }

    fn resolve_expr<'a>(&'a self, handle: &'a Self::ExprHandle) -> &'a ExprKind<Self> {
        &handle.0
    }
}

// --- IMPL 3: Ref Context (For Test Assertions) ---

// FIX: Explicit struct wrapper
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefHandle(pub &'static ExprKind<RefCtx>);

// Helper for "Copy" semantics on the reference
impl Copy for RefHandle {}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct RefCtx;

impl Context for RefCtx {
    type ExprHandle = RefHandle;

    fn alloc_expr(&mut self, _: Span, _: ExprKind<Self>) -> Self::ExprHandle {
        unimplemented!("RefCtx is read-only")
    }

    fn resolve_expr<'a>(&'a self, handle: &'a Self::ExprHandle) -> &'a ExprKind<Self> {
        handle.0
    }
}
