use crate::{
    parser::{AnnotatedSource, BinaryOp, BoolOp, ComparisonOp, UnaryOp},
    types::{
        Type,
        traits::{TypeKind, TypeView},
    },
    values::dynamic::Value,
};

extern crate hashbrown;
use hashbrown::{HashMap, DefaultHashBuilder};

/// Substitution from generalized type variable ID to concrete type
/// Uses arena allocation to avoid leaks when stored in arena-allocated structs
pub type Substitution<'types, 'arena> = HashMap<u16, &'types Type<'types>, DefaultHashBuilder, &'arena bumpalo::Bump>;

/// Track all instantiations of a specific polymorphic lambda
#[derive(Debug)]
pub struct LambdaInstantiations<'types, 'arena> {
    /// All unique substitutions observed for this lambda
    pub substitutions: alloc::vec::Vec<Substitution<'types, 'arena>>,
}

#[derive(Debug)]
pub struct TypedExpr<'types, 'arena> {
    pub expr: &'arena Expr<'types, 'arena>,
    pub ann: &'arena AnnotatedSource<'arena, Expr<'types, 'arena>>,
    /// Map from lambda expression pointer to its instantiation info
    /// This tracks how polymorphic lambdas are instantiated at different call sites
    /// Uses arena allocation to avoid leaks since TypedExpr is arena-allocated
    pub lambda_instantiations: HashMap<*const Expr<'types, 'arena>, LambdaInstantiations<'types, 'arena>, DefaultHashBuilder, &'arena bumpalo::Bump>,
}

#[derive(Debug, Clone)]
pub struct Expr<'types, 'arena>(pub &'types Type<'types>, pub ExprInner<'types, 'arena>);

impl<'types, 'arena> PartialEq for Expr<'types, 'arena> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.0, other.0) && self.1 == other.1
    }
}

impl<'types, 'arena> Eq for Expr<'types, 'arena> {}

impl<'types, 'arena> Expr<'types, 'arena> {
    pub fn as_ptr(&self) -> *const Self {
        self as *const _
    }

    /// Get a TypeView for pattern matching on this expression's type.
    pub fn type_view(&self) -> TypeKind<'types, &'types Type<'types>> {
        self.0.view()
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
    Comparison {
        op: ComparisonOp,
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
        captures: &'arena [&'arena str],
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
    /// Option constructor
    Option {
        inner: Option<&'arena Expr<'types, 'arena>>,
    },
    /// Pattern matching
    Match {
        expr: &'arena Expr<'types, 'arena>,
        arms: &'arena [TypedMatchArm<'types, 'arena>],
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

/// A typed pattern for matching and destructuring values.
#[derive(Debug, Clone, PartialEq)]
pub enum TypedPattern<'types, 'arena> {
    /// Wildcard pattern `_` - matches anything, binds nothing
    Wildcard,
    /// Variable pattern `x` - matches anything and binds to a name
    Var(&'arena str),
    /// Literal pattern - matches specific values
    Literal(Value<'types, 'arena>),
    /// Some pattern `some p` - matches Option::Some and destructures inner value
    Some(&'arena TypedPattern<'types, 'arena>),
    /// None pattern `none` - matches Option::None
    None,
}

/// A single arm in a typed match expression.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedMatchArm<'types, 'arena> {
    pub pattern: &'arena TypedPattern<'types, 'arena>,
    pub body: &'arena Expr<'types, 'arena>,
}
