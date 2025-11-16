use super::{Scalar, TypeBuilder, TypeKind};
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
/// }
///
/// impl<I: TypeBuilder> TypeVisitor<I> for DepthCalculator {
///     fn visit_ty(&mut self, ty: I::TypeView, builder: I) {
///         self.current_depth += 1;
///         self.max_depth = self.max_depth.max(self.current_depth);
///
///         self.super_visit_ty(ty, builder);
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
/// let mut calc = DepthCalculator { max_depth: 0, current_depth: 0 };
/// calc.visit_ty(arr, builder);
/// assert_eq!(calc.max_depth, 3); // Array -> Array -> Int
/// ```
pub trait TypeVisitor<B: TypeBuilder> {
    /// Visit a type.
    ///
    /// Override this to customize behavior for all types.
    /// Call `super_visit_ty` to recurse into nested types.
    fn visit_ty(&mut self, ty: B::TypeView, builder: B) {
        self.super_visit_ty(ty, builder)
    }

    /// Default recursion into nested types.
    ///
    /// Override `visit_ty` instead of this method.
    fn super_visit_ty(&mut self, ty: B::TypeView, builder: B) {
        match ty.view(builder) {
            // Base cases - no recursion
            TypeKind::TypeVar(_) | TypeKind::Scalar(_) | TypeKind::Symbol(_) => {}

            // Simple recursive cases
            TypeKind::Array(elem) => {
                self.visit_ty(elem.clone(), builder);
            }

            TypeKind::Map(key, val) => {
                self.visit_ty(key.clone(), builder);
                self.visit_ty(val.clone(), builder);
            }

            // Record: visit all field types
            TypeKind::Record(fields) => {
                for (_name, field_ty) in builder.field_types_data(fields) {
                    self.visit_ty(field_ty.clone(), builder);
                }
            }

            // Function: visit all param types and return type
            TypeKind::Function { params, ret } => {
                for param_ty in builder.types_data(params) {
                    self.visit_ty(param_ty.clone(), builder);
                }
                self.visit_ty(ret.clone(), builder);
            }
        }
    }
}

/// Example visitor: Count occurrences of Int type.
///
/// # Example
///
/// ```
/// use melbi_types::{IntCounter, TypeVisitor, BoxBuilder, TypeBuilder, Scalar, TypeKind};
///
/// let builder = BoxBuilder::new();
/// let arr_int = TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder);
///
/// let mut counter = IntCounter { count: 0 };
/// counter.visit_ty(arr_int, builder);
/// assert_eq!(counter.count, 1);
/// ```
pub struct IntCounter {
    pub count: usize,
}

impl<B: TypeBuilder> TypeVisitor<B> for IntCounter {
    fn visit_ty(&mut self, ty: B::TypeView, builder: B) {
        if matches!(ty.view(builder), TypeKind::Scalar(Scalar::Int)) {
            self.count += 1;
        }
        self.super_visit_ty(ty, builder);
    }
}
