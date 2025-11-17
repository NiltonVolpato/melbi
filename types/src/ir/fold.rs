use super::{TypeBuilder, TypeKind};
use crate::TypeView;
use alloc::vec::Vec;

/// Trait for transforming types.
///
/// Implement this trait to create new types based on existing ones.
/// The default implementation handles recursion automatically.
///
/// # Example
///
/// ```
/// use melbi_types::{TypeBuilder, Ty, TypeKind, Scalar, TypeFolder, BoxBuilder};
///
/// // Replace all Int with Bool
/// struct IntToBoolFolder {
///     builder: BoxBuilder,
/// }
///
/// impl TypeFolder<BoxBuilder> for IntToBoolFolder {
///     fn builder(&self) -> BoxBuilder {
///         self.builder
///     }
///
///     fn fold_ty(&mut self, ty: Ty<BoxBuilder>) -> Ty<BoxBuilder> {
///         match ty.kind(self.builder) {
///             TypeKind::Scalar(Scalar::Int) => TypeKind::Scalar(Scalar::Bool).intern(self.builder),
///             _ => self.super_fold_ty(ty),
///         }
///     }
/// }
///
/// let builder = BoxBuilder::new();
/// let arr_int = TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder);
///
/// let mut folder = IntToBoolFolder { builder };
/// let result = folder.fold_ty(arr_int);
/// // result is now Array[Bool]
/// assert!(match result.kind(builder) {
///     TypeKind::Array(elem) => elem.is_bool(builder),
///     _ => false,
/// });
/// ```
pub trait TypeFolder<B: TypeBuilder> {
    /// Get the interner for creating new types.
    fn builder(&self) -> B;

    /// Transform a type.
    ///
    /// Override this to customize behavior for all types.
    /// Call `super_fold_ty` to recurse into nested types.
    fn fold_ty(&mut self, ty: B::TypeView) -> B::TypeView
    where
        B::TypeView: From<crate::Ty<B>>,
    {
        self.super_fold_ty(ty)
    }

    /// Default recursion into nested types.
    ///
    /// Override `fold_ty` instead of this method.
    fn super_fold_ty(&mut self, ty: B::TypeView) -> B::TypeView
    where
        B::TypeView: From<crate::Ty<B>>,
    {
        let builder = self.builder();

        match ty.view(builder) {
            // Base cases - return as-is (re-interned for consistency)
            TypeKind::TypeVar(id) => TypeKind::TypeVar(*id).intern(builder).into(),
            TypeKind::Scalar(s) => TypeKind::Scalar(*s).intern(builder).into(),
            TypeKind::Symbol(parts) => TypeKind::Symbol(parts.clone()).intern(builder).into(),

            // Recursive cases - extract data into owned values, then fold
            TypeKind::Array(elem) => {
                let elem = elem.clone();
                let new_elem = self.fold_ty(elem);
                TypeKind::Array(new_elem).intern(builder).into()
            }

            TypeKind::Map(key, val) => {
                let key = key.clone();
                let val = val.clone();
                let new_key = self.fold_ty(key);
                let new_val = self.fold_ty(val);
                TypeKind::Map(new_key, new_val).intern(builder).into()
            }

            TypeKind::Record(fields) => {
                // Extract field data, fold types, and collect
                // We keep the strings owned so we can pass them to intern_field_types
                let folded_fields: Vec<(alloc::string::String, B::TypeView)> = builder
                    .field_types_data(fields)
                    .iter()
                    .map(|(name, field_ty)| {
                        let owned_name = alloc::string::String::from(name.as_ref());
                        let folded_ty = self.fold_ty(field_ty.clone());
                        (owned_name, folded_ty)
                    })
                    .collect();

                // Pass the owned strings directly - intern_field_types accepts AsRef<str>
                TypeKind::Record(builder.intern_field_types(folded_fields))
                    .intern(builder)
                    .into()
            }

            TypeKind::Function { params, ret } => {
                // Extract and own all param data before folding
                let param_data: Vec<B::TypeView> =
                    builder.types_data(params).iter().cloned().collect();
                let ret = ret.clone();

                // Now fold
                let new_params: Vec<B::TypeView> = param_data
                    .into_iter()
                    .map(|param_ty| self.fold_ty(param_ty))
                    .collect();
                let new_ret = self.fold_ty(ret);

                TypeKind::Function {
                    params: builder.intern_types(new_params),
                    ret: new_ret,
                }
                .intern(builder)
                .into()
            }
        }
    }
}
