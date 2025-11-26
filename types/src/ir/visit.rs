use super::TypeBuilder;
use crate::TypeView;

/// Trait for visiting types.
///
/// Implement this trait to traverse types without mutation.
/// The default implementation handles recursion automatically.
///
/// # Example
///
/// ```
/// use melbi_types::{TypeBuilder, Scalar, TypeKind, TypeVisitor, BoxBuilder};
///
/// struct DepthCalculator {
///     max_depth: usize,
///     current_depth: usize,
///     builder: BoxBuilder,
/// }
///
/// impl TypeVisitor<BoxBuilder> for DepthCalculator {
///     fn builder(&self) -> BoxBuilder {
///         self.builder
///     }
///
///     fn visit(&mut self, ty: <BoxBuilder as TypeBuilder>::TypeView) {
///         self.current_depth += 1;
///         self.max_depth = self.max_depth.max(self.current_depth);
///
///         self.super_visit(ty);
///
///         self.current_depth -= 1;
///     }
/// }
///
/// let builder = BoxBuilder::new();
/// let arr = TypeKind::Array(
///     TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder)
/// ).intern(builder);
///
/// let mut calc = DepthCalculator { max_depth: 0, current_depth: 0, builder };
/// calc.visit(arr);
/// assert_eq!(calc.max_depth, 3); // Array -> Array -> Int
/// ```
pub trait TypeVisitor<B: TypeBuilder> {
    /// Get the builder for this visitor.
    fn builder(&self) -> B;

    /// Visit a type.
    ///
    /// Override this to customize behavior for all types.
    /// Call `super_visit` to recurse into nested types.
    fn visit(&mut self, ty: B::TypeView) {
        self.super_visit(ty)
    }

    /// Default recursion into nested types.
    ///
    /// Override `visit` instead of this method.
    fn super_visit(&mut self, ty: B::TypeView) {
        let builder = self.builder();
        for child in ty.children(builder) {
            self.visit(child);
        }
    }
}

/// Visitor that delegates to a closure.
///
/// The closure receives each visited type and returns `true` if it handled
/// the node (stopping recursion), or `false` to continue with default traversal.
///
/// # Example
///
/// ```
/// use melbi_types::{ClosureVisitor, TypeVisitor, TypeView, BoxBuilder, TypeBuilder, Scalar, TypeKind};
///
/// let builder = BoxBuilder::new();
/// let arr = TypeKind::Array(
///     TypeKind::Map(
///         TypeKind::Scalar(Scalar::Int).intern(builder),
///         TypeKind::Scalar(Scalar::Bool).intern(builder)
///     ).intern(builder)
/// ).intern(builder);
///
/// let mut found_map = false;
/// let mut visitor = ClosureVisitor::new(builder, |ty| {
///     if matches!(ty.view(builder), TypeKind::Map(..)) {
///         found_map = true;
///         true  // Stop recursion
///     } else {
///         false  // Continue
///     }
/// });
///
/// visitor.visit(arr);
/// assert!(found_map);
/// ```
pub struct ClosureVisitor<B, F>
where
    B: TypeBuilder,
    F: FnMut(B::TypeView) -> bool,
{
    builder: B,
    closure: F,
}

impl<B, F> ClosureVisitor<B, F>
where
    B: TypeBuilder,
    F: FnMut(B::TypeView) -> bool,
{
    /// Create a new closure visitor.
    pub fn new(builder: B, closure: F) -> Self {
        Self { builder, closure }
    }
}

impl<B, F> TypeVisitor<B> for ClosureVisitor<B, F>
where
    B: TypeBuilder,
    F: FnMut(B::TypeView) -> bool,
{
    fn builder(&self) -> B {
        self.builder
    }

    fn visit(&mut self, ty: B::TypeView) {
        if !(self.closure)(ty.clone()) {
            // Closure returned false, use default traversal
            self.super_visit(ty);
        }
    }
}
