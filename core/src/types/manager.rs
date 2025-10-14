use std::fmt::Display;

use bumpalo::Bump;
use hashbrown::DefaultHashBuilder;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type<'a> {
    // Primitives.
    Int,
    Float,
    Bool,
    Str,
    Bytes,

    // Collections.
    Array(&'a Type<'a>),
    Map(&'a Type<'a>, &'a Type<'a>),

    // Structural records.
    Record(&'a [(&'a str, &'a Type<'a>)]), // Must be sorted by field name.

    // Functions.
    Function {
        params: &'a [&'a Type<'a>],
        ret: &'a Type<'a>,
    },

    // Symbols.
    Symbol(&'a [&'a str]), // Must be sorted.

    // Type variables.
    TypeVar(&'a str),
    // TODO: More types to add later:
    //   Custom(&'a str),
    //   Union(&'a [&'a Type<'a>]),  // Must be sorted.
}

impl Display for Type<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Bool => write!(f, "Bool"),
            Type::Str => write!(f, "Str"),
            Type::Bytes => write!(f, "Bytes"),
            Type::Array(elem_ty) => write!(f, "Array[{}]", elem_ty),
            Type::Map(key_ty, val_ty) => write!(f, "Map[{}, {}]", key_ty, val_ty),
            Type::Record(fields) => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(name, ty)| format!("{}: {}", name, ty))
                    .collect();
                write!(f, "Record[{}]", field_strs.join(", "))
            }
            Type::Function { params, ret } => {
                let param_strs: Vec<String> = params.iter().map(|ty| format!("{}", ty)).collect();
                write!(f, "({}) => {}", param_strs.join(", "), ret)
            }
            Type::Symbol(parts) => {
                let part_strs: Vec<String> = parts.iter().map(|p| p.to_string()).collect();
                write!(f, "Symbol[{}]", part_strs.join("|"))
            }
            Type::TypeVar(name) => write!(f, "TypeVar[{}]", name),
        }
    }
}

pub struct TypeManager<'a> {
    // Arena holding all types from this TypeManager.
    arena: &'a Bump,
    // Interned types to ensure uniqueness.
    interned: hashbrown::HashMap<Type<'a>, &'a Type<'a>, DefaultHashBuilder, &'a Bump>,
}

impl<'a> TypeManager<'a> {
    pub fn new(arena: &'a Bump) -> Self {
        Self {
            arena,
            interned: hashbrown::HashMap::new_in(arena),
        }
    }

    // Intern a type, returning a reference to the unique instance.
    fn intern(&mut self, ty: Type<'a>) -> &'a Type<'a> {
        if let Some(&interned_ty) = self.interned.get(&ty) {
            return interned_ty;
        }
        let arena_ty = self.arena.alloc(ty);
        self.interned.insert(arena_ty.clone(), arena_ty);
        arena_ty
    }

    // Factory methods for types.
    pub fn int(&mut self) -> &'a Type<'a> {
        self.intern(Type::Int)
    }
    pub fn float(&mut self) -> &'a Type<'a> {
        self.intern(Type::Float)
    }
    pub fn bool(&mut self) -> &'a Type<'a> {
        self.intern(Type::Bool)
    }
    pub fn str(&mut self) -> &'a Type<'a> {
        self.intern(Type::Str)
    }
    pub fn bytes(&mut self) -> &'a Type<'a> {
        self.intern(Type::Bytes)
    }
    pub fn array(&mut self, elem_ty: &'a Type<'a>) -> &'a Type<'a> {
        self.intern(Type::Array(elem_ty))
    }
    pub fn map(&mut self, key_ty: &'a Type<'a>, val_ty: &'a Type<'a>) -> &'a Type<'a> {
        self.intern(Type::Map(key_ty, val_ty))
    }
    pub fn record(&mut self, fields: &[(&str, &'a Type<'a>)]) -> &'a Type<'a> {
        // Ensure fields are sorted by name for uniqueness.
        let mut sorted_fields: Vec<(&'a str, &'a Type<'a>)> = fields
            .iter()
            .map(|(n, t)| (&*self.arena.alloc_str(*n), *t))
            .collect::<Vec<_>>();
        sorted_fields.sort_by_key(|(name, _)| *name);
        self.intern(Type::Record(self.arena.alloc_slice_copy(&sorted_fields)))
    }
    pub fn function(&mut self, params: &[&'a Type<'a>], ret: &'a Type<'a>) -> &'a Type<'a> {
        self.intern(Type::Function {
            params: self.arena.alloc_slice_copy(params),
            ret,
        })
    }
    pub fn symbol(&mut self, parts: &[&str]) -> &'a Type<'a> {
        let mut sorted_parts: Vec<&'a str> = parts
            .iter()
            .map(|p| &*self.arena.alloc_str(*p))
            .collect::<Vec<_>>();
        sorted_parts.sort();
        self.intern(Type::Symbol(self.arena.alloc_slice_copy(&sorted_parts)))
    }

    // TODO: Implement custom types and their capabilities.
    // pub fn custom(&mut self, name: String) -> &'a Type<'a> {
    //     self.intern(Type::Custom { name })
    // }

    // // Register a custom type's capabilities
    // pub fn register_custom_type<T: TypeCapabilities + 'static>(&mut self, capabilities: T) {
    //     self.type_registry.register_type(capabilities);
    // }

    // // Check if a custom type supports an operation
    // pub fn custom_type_supports(&self, type_name: &str, operation: &str) -> bool {
    //     self.type_registry.supports_operation(type_name, operation)
    // }

    // // Get capabilities for a custom type
    // pub fn get_custom_capabilities(&self, type_name: &str) -> Option<&dyn TypeCapabilities> {
    //     self.type_registry.get_capabilities(type_name)
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interning() {
        let bump = Bump::new();
        let mut manager = TypeManager::new(&bump);

        let int_type = manager.int();
        let float_type = manager.float();

        assert_eq!(int_type, manager.intern(Type::Int));
        assert_eq!(float_type, manager.intern(Type::Float));
    }
}
