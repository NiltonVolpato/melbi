use super::{Scalar, TypeBuilder};
use crate::TypeView;
use core::fmt;

// Import bitflags
use bitflags::bitflags;

bitflags! {
    /// Flags indicating various properties of a type.
    ///
    /// These flags are computed once when a type is interned and cached
    /// for efficient queries. This avoids repeated recursive traversals.
    ///
    /// Starting with an empty set - flags will be added as we implement
    /// features that need them (inference vars, placeholders, bound vars, etc.)
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub struct TypeFlags: u16 {
        // Future flags will be added here as features are implemented:
        // const HAS_TY_INFER = 1;           // when we add inference vars
        // const HAS_ERROR = 1 << 1;         // when we add error types
        // const NEEDS_SHIFT = 1 << 2;       // when we add bound vars
        // const HAS_TY_PLACEHOLDER = 1 << 3; // when we add placeholders
    }
}

/// Data for a type: kind + cached flags.
///
/// Following Chalk's design, this separates the type structure (kind)
/// from cached metadata (flags). The interner computes flags once
/// during interning.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TyData<B: TypeBuilder> {
    /// The actual type structure
    pub kind: TypeKind<B>,

    /// Cached properties for efficient queries
    pub flags: TypeFlags,
}

/// Logical structure of a type.
///
/// This is generic over the `Interner` so the same type kind works
/// with different storage strategies.
///
/// Following Chalk's pattern, scalar types (Bool, Int, Float) are
/// consolidated into a single Scalar variant rather than separate
/// enum variants.
///
/// Matches Melbi's type system from core/src/types/types.rs:
/// - TypeVar for unification variables
/// - Scalar types (Int, Float, Bool, Str, Bytes)
/// - Collections (Array, Map)
/// - Structural types (Record, Function)
/// - Symbol (tagged unions)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeKind<B: TypeBuilder> {
    /// Type variable for unification (Hindley-Milner style).
    ///
    /// Unlike Chalk's InferenceVar/Placeholder, Melbi uses simple
    /// numeric IDs for unification variables.
    TypeVar(u16),

    /// Scalar types (Bool, Int, Float, Str, Bytes)
    Scalar(Scalar),

    /// Array type with element type
    Array(B::TypeView),

    /// Map type with key and value types
    Map(B::TypeView, B::TypeView),

    /// Record (struct) with named fields.
    ///
    /// Fields are stored sorted by name for canonical representation.
    /// Field names are interned strings for efficient comparison.
    Record(B::InternedFieldTypes),

    /// Function type with parameters and return type.
    ///
    /// Parameters are stored as an interned list of types.
    Function {
        params: B::InternedTypes,
        ret: B::TypeView,
    },

    /// Symbol (tagged union) with sorted parts.
    ///
    /// Parts are interned strings stored in sorted order.
    /// Example: Symbol["error", "pending", "success"]
    Symbol(B::InternedSymbolParts),
}

impl<B: TypeBuilder> TypeKind<B> {
    /// Compute type flags for this type kind.
    ///
    /// This is called by the interner during type creation to build
    /// the TyData. Flags are cached to avoid repeated traversals.
    pub fn compute_flags(&self, builder: B) -> TypeFlags {
        match self {
            // TypeVar and Scalar types have no special flags
            TypeKind::TypeVar(_) | TypeKind::Scalar(_) => TypeFlags::empty(),

            // Array types inherit flags from their element type
            TypeKind::Array(elem) => elem.data(builder).flags,

            // Map types inherit flags from both key and value types
            TypeKind::Map(key, val) => key.data(builder).flags | val.data(builder).flags,

            // Record types inherit flags from all field types
            TypeKind::Record(fields) => {
                let mut flags = TypeFlags::empty();
                for (_name, field_ty) in builder.field_types_data(fields) {
                    flags |= field_ty.data(builder).flags;
                }
                flags
            }

            // Function types inherit flags from params and return type
            TypeKind::Function { params, ret } => {
                let mut flags = ret.data(builder).flags;
                for param in builder.types_data(params) {
                    flags |= param.data(builder).flags;
                }
                flags
            }

            // Symbol types have no special flags (just strings)
            TypeKind::Symbol(_) => TypeFlags::empty(),
        }
    }

    /// Intern this type kind into a Ty handle.
    ///
    /// This is a convenience method that computes flags and calls
    /// the builder's intern_ty method.
    pub fn intern(self, builder: B) -> Ty<B> {
        Ty::new(builder.intern_ty(self))
    }
}

/// Handle to an interned type.
///
/// This is a lightweight wrapper around the interner's representation.
/// It can be cloned and used to retrieve the full type data.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Ty<B: TypeBuilder> {
    interned: B::InternedTy,
}

// Implement Copy when InternedTy is Copy (e.g., for ArenaBuilder)
impl<B: TypeBuilder> Copy for Ty<B> where B::InternedTy: Copy {}

impl<B: TypeBuilder> Ty<B> {
    /// Create a new type from an interned handle.
    pub fn new(interned: B::InternedTy) -> Self {
        Self { interned }
    }

    /// Get the interned representation (for internal use).
    pub fn interned(&self) -> &B::InternedTy {
        &self.interned
    }

    /// Get the full type data (kind + flags) by looking up in the interner.
    pub fn data(&self, builder: B) -> &TyData<B> {
        builder.ty_data(&self.interned)
    }

    /// Get the type kind by looking up in the interner.
    pub fn kind(&self, builder: B) -> &TypeKind<B> {
        &self.data(builder).kind
    }

    /// Check if this is a type variable.
    pub fn is_type_var(&self, builder: B) -> bool {
        matches!(self.kind(builder), TypeKind::TypeVar(_))
    }

    /// Check if this is an array type.
    pub fn is_array(&self, builder: B) -> bool {
        matches!(self.kind(builder), TypeKind::Array(_))
    }

    /// Check if this is a map type.
    pub fn is_map(&self, builder: B) -> bool {
        matches!(self.kind(builder), TypeKind::Map(_, _))
    }

    /// Check if this is a record type.
    pub fn is_record(&self, builder: B) -> bool {
        matches!(self.kind(builder), TypeKind::Record(_))
    }

    /// Check if this is a function type.
    pub fn is_function(&self, builder: B) -> bool {
        matches!(self.kind(builder), TypeKind::Function { .. })
    }

    /// Check if this is a symbol type.
    pub fn is_symbol(&self, builder: B) -> bool {
        matches!(self.kind(builder), TypeKind::Symbol(_))
    }

    /// Check if this is a scalar type.
    pub fn is_scalar(&self, builder: B) -> bool {
        matches!(self.kind(builder), TypeKind::Scalar(_))
    }

    /// Check if this is the Int type.
    pub fn is_int(&self, builder: B) -> bool {
        matches!(self.kind(builder), TypeKind::Scalar(Scalar::Int))
    }

    /// Check if this is the Bool type.
    pub fn is_bool(&self, builder: B) -> bool {
        matches!(self.kind(builder), TypeKind::Scalar(Scalar::Bool))
    }

    /// Check if this is the Float type.
    pub fn is_float(&self, builder: B) -> bool {
        matches!(self.kind(builder), TypeKind::Scalar(Scalar::Float))
    }
}

impl<B: TypeBuilder> fmt::Debug for Ty<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ty({:?})", self.interned)
    }
}
