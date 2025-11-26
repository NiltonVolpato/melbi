use super::{Scalar, TypeBuilder, TypeKind, TypeVisitor};
use crate::TypeView;
use alloc::string::String;
use core::fmt::Write;

/// Visitor that formats types into strings.
///
/// This demonstrates using the visitor pattern for Display/Debug.
pub struct TypeFormatter<B: TypeBuilder> {
    output: String,
    builder: B,
}

impl<B: TypeBuilder> TypeFormatter<B> {
    pub fn new(builder: B) -> Self {
        Self {
            output: String::new(),
            builder,
        }
    }

    pub fn format(ty: B::TypeView, builder: B) -> String {
        let mut formatter = Self::new(builder);
        formatter.visit(ty);
        formatter.output
    }
}

impl<B: TypeBuilder> TypeVisitor<B> for TypeFormatter<B> {
    fn builder(&self) -> B {
        self.builder
    }

    fn visit(&mut self, ty: B::TypeView) {
        let builder = self.builder;
        match ty.view(builder) {
            TypeKind::TypeVar(id) => {
                let _ = write!(self.output, "_{}", id);
            }
            TypeKind::Scalar(Scalar::Int) => {
                let _ = write!(self.output, "Int");
            }
            TypeKind::Scalar(Scalar::Bool) => {
                let _ = write!(self.output, "Bool");
            }
            TypeKind::Scalar(Scalar::Float) => {
                let _ = write!(self.output, "Float");
            }
            TypeKind::Scalar(Scalar::Str) => {
                let _ = write!(self.output, "Str");
            }
            TypeKind::Scalar(Scalar::Bytes) => {
                let _ = write!(self.output, "Bytes");
            }
            TypeKind::Array(elem) => {
                let _ = write!(self.output, "Array[");
                self.visit(elem.clone());
                let _ = write!(self.output, "]");
            }
            TypeKind::Map(key, val) => {
                let _ = write!(self.output, "Map[");
                self.visit(key.clone());
                let _ = write!(self.output, ", ");
                self.visit(val.clone());
                let _ = write!(self.output, "]");
            }
            TypeKind::Record(fields) => {
                let _ = write!(self.output, "Record[");
                let field_data = builder.field_types_data(fields);
                for (i, (name, field_ty)) in field_data.iter().enumerate() {
                    if i > 0 {
                        let _ = write!(self.output, ", ");
                    }
                    let _ = write!(self.output, "{}: ", name);
                    self.visit(field_ty.clone());
                }
                let _ = write!(self.output, "]");
            }
            TypeKind::Function { params, ret } => {
                let _ = write!(self.output, "(");
                let param_data = builder.types_data(params);
                for (i, param_ty) in param_data.iter().enumerate() {
                    if i > 0 {
                        let _ = write!(self.output, ", ");
                    }
                    self.visit(param_ty.clone());
                }
                let _ = write!(self.output, ") => ");
                self.visit(ret.clone());
            }
            TypeKind::Symbol(parts) => {
                let _ = write!(self.output, "Symbol[");
                let part_data = builder.symbol_parts_data(parts);
                for (i, part) in part_data.iter().enumerate() {
                    if i > 0 {
                        let _ = write!(self.output, "|");
                    }
                    let _ = write!(self.output, "{}", part);
                }
                let _ = write!(self.output, "]");
            }
        }
    }
}

/// Extension trait to add display methods to TyKind.
pub trait TypeKindDisplay<B: TypeBuilder> {
    fn display(&self, builder: B) -> String;
}

impl<B: TypeBuilder> TypeKindDisplay<B> for TypeKind<B>
where
    B::TypeView: From<crate::Ty<B>>,
{
    fn display(&self, builder: B) -> String {
        // Just use the intern and format pattern for simplicity
        TypeFormatter::format(self.clone().intern(builder).into(), builder)
    }
}

/// Extension trait to add display methods to TypeView.
pub trait TyDisplay<B: TypeBuilder> {
    fn display(&self, builder: B) -> String;
}

impl<B: TypeBuilder> TyDisplay<B> for B::TypeView {
    fn display(&self, builder: B) -> String {
        TypeFormatter::format(self.clone(), builder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoxBuilder, Scalar, TypeKind};

    #[test]
    fn test_format_int() {
        let builder = BoxBuilder::new();
        let ty = TypeKind::Scalar(Scalar::Int).intern(builder);
        assert_eq!(TypeFormatter::format(ty, builder), "Int");
    }

    #[test]
    fn test_format_bool() {
        let builder = BoxBuilder::new();
        let ty = TypeKind::Scalar(Scalar::Bool).intern(builder);
        assert_eq!(TypeFormatter::format(ty, builder), "Bool");
    }

    #[test]
    fn test_format_float() {
        let builder = BoxBuilder::new();
        let ty = TypeKind::Scalar(Scalar::Float).intern(builder);
        assert_eq!(TypeFormatter::format(ty, builder), "Float");
    }

    #[test]
    fn test_format_array() {
        let builder = BoxBuilder::new();
        let ty = TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder);
        assert_eq!(TypeFormatter::format(ty, builder), "Array[Int]");
    }

    #[test]
    fn test_format_nested_array() {
        let builder = BoxBuilder::new();
        let inner = TypeKind::Array(TypeKind::Scalar(Scalar::Bool).intern(builder)).intern(builder);
        let outer = TypeKind::Array(inner).intern(builder);
        assert_eq!(TypeFormatter::format(outer, builder), "Array[Array[Bool]]");
    }

    #[test]
    fn test_display_extension() {
        let builder = BoxBuilder::new();
        let ty = TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder);
        assert_eq!(ty.display(builder), "Array[Int]");
    }
}
