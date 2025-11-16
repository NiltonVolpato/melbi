//! Scalar type variants for Melbi.
//!
//! This module defines the Scalar enum which represents all primitive scalar types
//! in Melbi's type system, following Chalk's pattern of consolidating scalar types
//! into a single enum.

/// Scalar type variants
///
/// Unlike Chalk (which models Rust and needs Int/Uint/Float distinctions),
/// Melbi uses a simpler set of scalar types appropriate for its domain.
///
/// Matches Melbi's core type system from core/src/types/types.rs.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scalar {
    /// Boolean type
    Bool,

    /// Integer type (unified, no signed/unsigned distinction)
    Int,

    /// Floating-point type (unified, no size distinction)
    Float,

    /// String type
    Str,

    /// Bytes type
    Bytes,
}

impl Scalar {
    /// Returns true if this scalar is a numeric type (Int or Float)
    pub fn is_numeric(&self) -> bool {
        matches!(self, Scalar::Int | Scalar::Float)
    }

    /// Returns true if this scalar supports comparison operations
    pub fn is_comparable(&self) -> bool {
        // All scalars except Bytes are comparable
        !matches!(self, Scalar::Bytes)
    }

    /// Returns true if this scalar is a string-like type (Str or Bytes)
    pub fn is_string_like(&self) -> bool {
        matches!(self, Scalar::Str | Scalar::Bytes)
    }
}
