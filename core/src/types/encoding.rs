//! # Melbi Type Encoding
//!
//! Compact binary encoding for Melbi types with zero-copy navigation.
//!
//! ## Overview
//!
//! This module provides an alternate representation for Melbi types that:
//! - Has no lifetime dependencies (just `&[u8]` or `Vec<u8>`)
//! - Supports fast equality via byte comparison
//! - Enables zero-copy navigation via `TypeView`
//! - Roundtrips perfectly with arena representation
//!
//! ## Format
//!
//! Types are encoded with a discriminant byte indicating the variant:
//!
//! - **0-31**: Packed TypeVar(0..31)
//! - **32-63**: Unitary types (Int, Float, Bool, Str, Bytes)
//! - **64-95**: Composite types (Array, Map, Record, Function, Symbol)
//! - **96-127**: Packed types (Array[Primitive], Map[Primitive, Primitive])
//!
//! Composite types include a u16 size field: `[disc][size_lo][size_hi][payload...]`
//!
//! See the design document for full specification.

use crate::{Type, Vec};
use core::fmt;

// ============================================================================
// Discriminant Constants
// ============================================================================

// TypeVar: 0-31 (packed for IDs 0-31)
const TYPEVAR_BASE: u8 = 0;
const TYPEVAR_MAX_PACKED: u16 = 31;

// Unitary: 32-63
const DISC_INT: u8 = 32;
const DISC_FLOAT: u8 = 33;
const DISC_BOOL: u8 = 34;
const DISC_STR: u8 = 35;
const DISC_BYTES: u8 = 36;

// Composite: 64-95
const DISC_ARRAY: u8 = 64;
const DISC_MAP: u8 = 65;
const DISC_RECORD: u8 = 66;
const DISC_FUNCTION: u8 = 67;
const DISC_SYMBOL: u8 = 68;
const DISC_TYPEVAR: u8 = 69; // For TypeVar IDs >= 32

// Packed: 96-127
const DISC_ARRAY_INT: u8 = 96;
const DISC_ARRAY_FLOAT: u8 = 97;
const DISC_ARRAY_BOOL: u8 = 98;
const DISC_ARRAY_STR: u8 = 99;
const DISC_ARRAY_BYTES: u8 = 100;

const DISC_MAP_BASE: u8 = 101; // Maps: 101-125

// Categories (disc >> 5)
const CAT_TYPEVAR: u8 = 0; // 0-31
const CAT_UNITARY: u8 = 1; // 32-63
const CAT_COMPOSITE: u8 = 2; // 64-95
const CAT_PACKED: u8 = 3; // 96-127

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during decoding
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Hit end of buffer unexpectedly
    Truncated { offset: usize, needed: usize },

    /// Size field doesn't match actual content
    SizeMismatch {
        offset: usize,
        claimed: usize,
        actual: usize,
    },

    /// Invalid discriminant byte
    UnknownDiscriminant { discriminant: u8, offset: usize },

    /// Invalid UTF-8 in string
    InvalidUtf8 { offset: usize },

    /// Invalid varint encoding
    InvalidVarint { offset: usize },

    /// Exceeded maximum recursion depth
    TooDeep { depth: usize },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecodeError::Truncated { offset, needed } => {
                write!(
                    f,
                    "truncated at offset {}: need {} more bytes",
                    offset, needed
                )
            }
            DecodeError::SizeMismatch {
                offset,
                claimed,
                actual,
            } => {
                write!(
                    f,
                    "size mismatch at offset {}: claimed {} but actual {}",
                    offset, claimed, actual
                )
            }
            DecodeError::UnknownDiscriminant {
                discriminant,
                offset,
            } => {
                write!(
                    f,
                    "unknown discriminant {} at offset {}",
                    discriminant, offset
                )
            }
            DecodeError::InvalidUtf8 { offset } => {
                write!(f, "invalid UTF-8 at offset {}", offset)
            }
            DecodeError::InvalidVarint { offset } => {
                write!(f, "invalid varint at offset {}", offset)
            }
            DecodeError::TooDeep { depth } => {
                write!(f, "exceeded maximum recursion depth: {}", depth)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get discriminant for packed array
#[inline]
fn disc_array_packed(elem: &Type) -> Option<u8> {
    match elem {
        Type::Int => Some(DISC_ARRAY_INT),
        Type::Float => Some(DISC_ARRAY_FLOAT),
        Type::Bool => Some(DISC_ARRAY_BOOL),
        Type::Str => Some(DISC_ARRAY_STR),
        Type::Bytes => Some(DISC_ARRAY_BYTES),
        _ => None,
    }
}

/// Get discriminant for packed map (both key and value must be primitives)
#[inline]
fn disc_map_packed(key: &Type, val: &Type) -> Option<u8> {
    let key_idx = match key {
        Type::Int => 0,
        Type::Float => 1,
        Type::Bool => 2,
        Type::Str => 3,
        Type::Bytes => 4,
        _ => return None,
    };
    let val_idx = match val {
        Type::Int => 0,
        Type::Float => 1,
        Type::Bool => 2,
        Type::Str => 3,
        Type::Bytes => 4,
        _ => return None,
    };
    Some(DISC_MAP_BASE + key_idx * 5 + val_idx)
}

/// Get primitive type by index (0=Int, 1=Float, 2=Bool, 3=Str, 4=Bytes)
#[inline]
fn primitive_by_idx<'a>(idx: u8, mgr: &'a crate::types::manager::TypeManager<'a>) -> &'a Type<'a> {
    match idx {
        0 => mgr.int(),
        1 => mgr.float(),
        2 => mgr.bool(),
        3 => mgr.str(),
        4 => mgr.bytes(),
        _ => unreachable!("primitive index out of range"),
    }
}

/// Write a varint (LEB128 encoding)
fn write_varint(buf: &mut OutputVec, mut n: usize) {
    loop {
        let byte = (n & 0x7F) as u8;
        n >>= 7;
        if n == 0 {
            buf.push(byte);
            break;
        } else {
            buf.push(byte | 0x80);
        }
    }
}

/// Write a string (length-prefixed UTF-8)
fn write_string(buf: &mut OutputVec, s: &str) {
    write_varint(buf, s.len());
    buf.extend_from_slice(s.as_bytes());
}

/// Read a varint, returns (value, bytes_consumed)
fn read_varint(bytes: &[u8]) -> Result<(usize, usize), DecodeError> {
    let mut result = 0usize;
    let mut shift = 0;
    let mut pos = 0;

    while pos < bytes.len() {
        let byte = bytes[pos];
        pos += 1;

        result |= ((byte & 0x7F) as usize) << shift;

        if byte & 0x80 == 0 {
            return Ok((result, pos));
        }

        shift += 7;
        if shift >= 64 {
            return Err(DecodeError::InvalidVarint { offset: pos });
        }
    }

    Err(DecodeError::Truncated {
        offset: bytes.len(),
        needed: 1,
    })
}

/// Read a string, returns (string, bytes_consumed)
fn read_string(bytes: &[u8]) -> Result<(&str, usize), DecodeError> {
    let (len, varint_len) = read_varint(bytes)?;

    if bytes.len() < varint_len + len {
        return Err(DecodeError::Truncated {
            offset: bytes.len(),
            needed: varint_len + len,
        });
    }

    let str_bytes = &bytes[varint_len..varint_len + len];
    let s = core::str::from_utf8(str_bytes)
        .map_err(|_| DecodeError::InvalidUtf8 { offset: varint_len })?;

    Ok((s, varint_len + len))
}

/// Read u16 little-endian
#[inline]
fn read_u16_le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

/// Write u16 little-endian
#[inline]
fn write_u16_le(buf: &mut OutputVec, val: u16) {
    buf.extend_from_slice(&val.to_le_bytes());
}

type OutputVec = Vec<u8>;

// ============================================================================
// Encoding
// ============================================================================

/// Encode a type to bytes.
///
/// Most types fit in 16 bytes (no heap allocation).
/// Encoding is deterministic and canonical.
///
/// # Examples
///
/// ```
/// # use melbi_core::types::{manager::TypeManager, encoding::encode};
/// # use bumpalo::Bump;
/// let arena = Bump::new();
/// let mgr = TypeManager::new(&arena);
///
/// let ty = mgr.map(mgr.int(), mgr.str());
/// let bytes = encode(ty);
/// assert_eq!(bytes.len(), 1); // Packed map
/// ```
pub fn encode(ty: &Type) -> OutputVec {
    let mut buf = OutputVec::new();
    encode_inner(ty, &mut buf);
    buf
}

fn encode_inner(ty: &Type, buf: &mut OutputVec) {
    match ty {
        // TypeVar: try packed encoding first
        Type::TypeVar(id) if *id <= TYPEVAR_MAX_PACKED => {
            buf.push(TYPEVAR_BASE + (*id as u8));
        }

        // Primitives: unitary encoding
        Type::Int => buf.push(DISC_INT),
        Type::Float => buf.push(DISC_FLOAT),
        Type::Bool => buf.push(DISC_BOOL),
        Type::Str => buf.push(DISC_STR),
        Type::Bytes => buf.push(DISC_BYTES),

        // Array: try packed encoding
        Type::Array(elem) => {
            if let Some(disc) = disc_array_packed(elem) {
                buf.push(disc);
            } else {
                // [64][size:u16][elem]
                let start = buf.len();
                buf.push(DISC_ARRAY);
                buf.push(0); // size placeholder
                buf.push(0);
                encode_inner(elem, buf);
                // Backpatch size
                let size = buf.len() - start - 3;
                buf[start + 1] = (size & 0xFF) as u8;
                buf[start + 2] = ((size >> 8) & 0xFF) as u8;
            }
        }

        // Map: try packed encoding
        Type::Map(key, val) => {
            if let Some(disc) = disc_map_packed(key, val) {
                buf.push(disc);
            } else {
                // [65][size:u16][key][val]
                let start = buf.len();
                buf.push(DISC_MAP);
                buf.push(0); // size placeholder
                buf.push(0);
                encode_inner(key, buf);
                encode_inner(val, buf);
                // Backpatch size
                let size = buf.len() - start - 3;
                buf[start + 1] = (size & 0xFF) as u8;
                buf[start + 2] = ((size >> 8) & 0xFF) as u8;
            }
        }

        // [66][size:u16][count:varint]([name_len:varint][name][type])*
        Type::Record(fields) => {
            let start = buf.len();
            buf.push(DISC_RECORD);
            buf.push(0); // size placeholder
            buf.push(0);
            write_varint(buf, fields.len());
            for (name, ty) in fields.iter() {
                write_string(buf, name);
                encode_inner(ty, buf);
            }
            // Backpatch size
            let size = buf.len() - start - 3;
            buf[start + 1] = (size & 0xFF) as u8;
            buf[start + 2] = ((size >> 8) & 0xFF) as u8;
        }

        // [67][size:u16][param_count:varint]([param])*[ret]
        Type::Function { params, ret } => {
            let start = buf.len();
            buf.push(DISC_FUNCTION);
            buf.push(0); // size placeholder
            buf.push(0);
            write_varint(buf, params.len());
            for param in params.iter() {
                encode_inner(param, buf);
            }
            encode_inner(ret, buf);
            // Backpatch size
            let size = buf.len() - start - 3;
            buf[start + 1] = (size & 0xFF) as u8;
            buf[start + 2] = ((size >> 8) & 0xFF) as u8;
        }

        // [68][size:u16][part_count:varint]([part_len:varint][part])*
        Type::Symbol(parts) => {
            let start = buf.len();
            buf.push(DISC_SYMBOL);
            buf.push(0); // size placeholder
            buf.push(0);
            write_varint(buf, parts.len());
            for part in parts.iter() {
                write_string(buf, part);
            }
            // Backpatch size
            let size = buf.len() - start - 3;
            buf[start + 1] = (size & 0xFF) as u8;
            buf[start + 2] = ((size >> 8) & 0xFF) as u8;
        }

        // [69][size:u16][id_lo][id_hi]
        Type::TypeVar(id) => {
            buf.push(DISC_TYPEVAR);
            buf.push(2); // size = 2 bytes for u16
            buf.push(0);
            write_u16_le(buf, *id);
        }
    }
}

// ============================================================================
// TypeView
// ============================================================================

/// Zero-copy view over encoded type bytes.
///
/// Provides efficient navigation and pattern matching without deserialization.
///
/// # Examples
///
/// ```
/// # use melbi_core::types::{manager::TypeManager, encoding::{encode, TypeView}};
/// # use bumpalo::Bump;
/// let arena = Bump::new();
/// let mgr = TypeManager::new(&arena);
///
/// let ty = mgr.map(mgr.array(mgr.int()), mgr.str());
/// let bytes = encode(ty);
/// let view = TypeView::new(&bytes);
///
/// if let Some((key, val)) = view.as_map() {
///     assert!(key.as_array().is_some());
///     assert_eq!(val.discriminant(), 35); // Str
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct TypeView<'a> {
    bytes: &'a [u8],
}

impl<'a> TypeView<'a> {
    /// Create a view from bytes without validation.
    ///
    /// # Safety
    ///
    /// Caller must ensure bytes contain a valid encoding.
    /// Use `validated()` for checked construction.
    #[inline]
    pub fn new(bytes: &'a [u8]) -> Self {
        TypeView { bytes }
    }

    /// Create a validated view, checking all invariants.
    pub fn validated(bytes: &'a [u8]) -> Result<Self, DecodeError> {
        validate_encoding(bytes, 0)?;
        Ok(TypeView { bytes })
    }

    /// Get the discriminant byte
    #[inline]
    pub fn discriminant(&self) -> u8 {
        self.bytes[0] & 0x7F
    }

    /// Get the category (0=TypeVar, 1=Unitary, 2=Composite, 3=Packed)
    #[inline]
    pub fn category(&self) -> u8 {
        self.discriminant() >> 5
    }

    /// Check if this is a unitary type (no children to navigate)
    #[inline]
    pub fn is_unitary(&self) -> bool {
        let cat = self.category();
        cat == CAT_TYPEVAR || cat == CAT_UNITARY || cat == CAT_PACKED
    }

    /// For composite types: get size of payload (excluding discriminant and size bytes)
    #[inline]
    pub fn size(&self) -> Option<u16> {
        if self.is_unitary() {
            None
        } else {
            Some(read_u16_le(&self.bytes[1..3]))
        }
    }

    /// Total encoded length in bytes
    #[inline]
    pub fn encoded_len(&self) -> usize {
        if self.is_unitary() {
            1
        } else {
            3 + self.size().unwrap() as usize
        }
    }

    /// Access payload bytes (for composite types)
    #[inline]
    fn payload(&self) -> &'a [u8] {
        if self.is_unitary() {
            &[]
        } else {
            let size = self.size().unwrap() as usize;
            &self.bytes[3..3 + size]
        }
    }

    /// If this is an array, return element type view.
    /// Works for both packed and non-packed arrays.
    pub fn as_array(&self) -> Option<TypeView<'a>> {
        let disc = self.discriminant();

        // Packed arrays
        if (DISC_ARRAY_INT..=DISC_ARRAY_BYTES).contains(&disc) {
            // Return a view of the implicit primitive
            // We need to synthesize the primitive byte
            // For now, return None since packed arrays are leaf nodes
            return None;
        }

        // Non-packed array: [64][size:u16][elem]
        if disc == DISC_ARRAY {
            Some(TypeView::new(self.payload()))
        } else {
            None
        }
    }

    /// If this is a map, return (key, value) views.
    /// Works for both packed and non-packed maps.
    pub fn as_map(&self) -> Option<(TypeView<'a>, TypeView<'a>)> {
        let disc = self.discriminant();

        // Packed maps
        if (DISC_MAP_BASE..=DISC_MAP_BASE + 24).contains(&disc) {
            // Packed maps are leaf nodes
            return None;
        }

        // Non-packed map: [65][size:u16][key][val]
        if disc == DISC_MAP {
            let payload = self.payload();
            let key = TypeView::new(payload);
            let key_len = key.encoded_len();
            let val = TypeView::new(&payload[key_len..]);
            Some((key, val))
        } else {
            None
        }
    }

    /// If this is a record, return iterator over fields.
    pub fn as_record(&self) -> Option<RecordIter<'a>> {
        if self.discriminant() == DISC_RECORD {
            let payload = self.payload();
            let (count, varint_len) = read_varint(payload).ok()?;
            Some(RecordIter {
                payload: &payload[varint_len..],
                remaining: count,
                pos: 0,
            })
        } else {
            None
        }
    }

    /// If this is a function, return function view with params iterator.
    pub fn as_function(&self) -> Option<FunctionView<'a>> {
        if self.discriminant() == DISC_FUNCTION {
            let payload = self.payload();
            let (param_count, varint_len) = read_varint(payload).ok()?;
            Some(FunctionView {
                payload,
                param_count,
                varint_start: varint_len,
            })
        } else {
            None
        }
    }

    /// If this is a symbol, return iterator over parts.
    pub fn as_symbol(&self) -> Option<SymbolIter<'a>> {
        if self.discriminant() == DISC_SYMBOL {
            let payload = self.payload();
            let (count, varint_len) = read_varint(payload).ok()?;
            Some(SymbolIter {
                payload: &payload[varint_len..],
                remaining: count,
                pos: 0,
            })
        } else {
            None
        }
    }

    /// If this is a type variable, return its ID.
    pub fn as_typevar(&self) -> Option<u16> {
        let disc = self.discriminant();
        let cat = self.category();

        if cat == CAT_TYPEVAR {
            // Packed typevar: 0-31
            Some(disc as u16)
        } else if disc == DISC_TYPEVAR {
            // Non-packed: [69][size:u16][id_lo][id_hi]
            let payload = self.payload();
            Some(read_u16_le(payload))
        } else {
            None
        }
    }
}

// ============================================================================
// Iterators
// ============================================================================

/// Iterator over record fields
pub struct RecordIter<'a> {
    payload: &'a [u8],
    remaining: usize,
    pos: usize,
}

impl<'a> Iterator for RecordIter<'a> {
    type Item = Result<(&'a str, TypeView<'a>), DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        self.remaining -= 1;

        // Read field name
        let name_result = read_string(&self.payload[self.pos..]);
        let (name, name_len) = match name_result {
            Ok(r) => r,
            Err(e) => return Some(Err(e)),
        };
        self.pos += name_len;

        // Read field type
        let ty_view = TypeView::new(&self.payload[self.pos..]);
        let ty_len = ty_view.encoded_len();
        self.pos += ty_len;

        Some(Ok((name, ty_view)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a> ExactSizeIterator for RecordIter<'a> {}

/// View of a function type with params iterator
pub struct FunctionView<'a> {
    payload: &'a [u8],
    param_count: usize,
    varint_start: usize,
}

impl<'a> FunctionView<'a> {
    /// Iterator over parameter types
    pub fn params(&self) -> ParamsIter<'a> {
        ParamsIter {
            payload: &self.payload[self.varint_start..],
            remaining: self.param_count,
            pos: 0,
        }
    }

    /// Get return type (computed by skipping all params)
    pub fn return_type(&self) -> Result<TypeView<'a>, DecodeError> {
        let mut pos = self.varint_start;
        for _ in 0..self.param_count {
            let view = TypeView::new(&self.payload[pos..]);
            pos += view.encoded_len();
        }

        if pos >= self.payload.len() {
            return Err(DecodeError::Truncated {
                offset: pos,
                needed: 1,
            });
        }

        Ok(TypeView::new(&self.payload[pos..]))
    }
}

/// Iterator over function parameters
pub struct ParamsIter<'a> {
    payload: &'a [u8],
    remaining: usize,
    pos: usize,
}

impl<'a> Iterator for ParamsIter<'a> {
    type Item = Result<TypeView<'a>, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        self.remaining -= 1;

        if self.pos >= self.payload.len() {
            return Some(Err(DecodeError::Truncated {
                offset: self.pos,
                needed: 1,
            }));
        }

        let view = TypeView::new(&self.payload[self.pos..]);
        let len = view.encoded_len();
        self.pos += len;

        Some(Ok(view))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a> ExactSizeIterator for ParamsIter<'a> {}

/// Iterator over symbol parts
pub struct SymbolIter<'a> {
    payload: &'a [u8],
    remaining: usize,
    pos: usize,
}

impl<'a> Iterator for SymbolIter<'a> {
    type Item = Result<&'a str, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        self.remaining -= 1;

        let result = read_string(&self.payload[self.pos..]);
        match result {
            Ok((part, part_len)) => {
                self.pos += part_len;
                Some(Ok(part))
            }
            Err(e) => Some(Err(e)),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a> ExactSizeIterator for SymbolIter<'a> {}

// ============================================================================
// Validation
// ============================================================================

/// Validate an encoding, checking all invariants.
/// Returns the total encoded length.
fn validate_encoding(bytes: &[u8], depth: usize) -> Result<usize, DecodeError> {
    const MAX_DEPTH: usize = 1000;

    if depth > MAX_DEPTH {
        return Err(DecodeError::TooDeep { depth });
    }

    if bytes.is_empty() {
        return Err(DecodeError::Truncated {
            offset: 0,
            needed: 1,
        });
    }

    let disc = bytes[0] & 0x7F;
    let cat = disc >> 5;

    match cat {
        CAT_TYPEVAR => {
            // Packed typevar: single byte
            Ok(1)
        }
        CAT_UNITARY => {
            // Primitives: single byte
            match disc {
                DISC_INT | DISC_FLOAT | DISC_BOOL | DISC_STR | DISC_BYTES => Ok(1),
                _ => Err(DecodeError::UnknownDiscriminant {
                    discriminant: disc,
                    offset: 0,
                }),
            }
        }
        CAT_COMPOSITE => {
            // Need size field
            if bytes.len() < 3 {
                return Err(DecodeError::Truncated {
                    offset: 0,
                    needed: 3,
                });
            }

            let size = read_u16_le(&bytes[1..3]) as usize;

            if bytes.len() < 3 + size {
                return Err(DecodeError::Truncated {
                    offset: bytes.len(),
                    needed: 3 + size,
                });
            }

            let payload = &bytes[3..3 + size];

            match disc {
                DISC_ARRAY => {
                    // Validate element
                    let elem_len = validate_encoding(payload, depth + 1)?;
                    if elem_len != size {
                        return Err(DecodeError::SizeMismatch {
                            offset: 0,
                            claimed: size,
                            actual: elem_len,
                        });
                    }
                    Ok(3 + size)
                }
                DISC_MAP => {
                    // Validate key and value
                    let key_len = validate_encoding(payload, depth + 1)?;
                    let val_len = validate_encoding(&payload[key_len..], depth + 1)?;
                    if key_len + val_len != size {
                        return Err(DecodeError::SizeMismatch {
                            offset: 0,
                            claimed: size,
                            actual: key_len + val_len,
                        });
                    }
                    Ok(3 + size)
                }
                DISC_RECORD => {
                    // Validate field count and fields
                    let (field_count, mut pos) = read_varint(payload)?;
                    for _ in 0..field_count {
                        let (_, name_len) = read_string(&payload[pos..])?;
                        pos += name_len;
                        let ty_len = validate_encoding(&payload[pos..], depth + 1)?;
                        pos += ty_len;
                    }
                    if pos != size {
                        return Err(DecodeError::SizeMismatch {
                            offset: 0,
                            claimed: size,
                            actual: pos,
                        });
                    }
                    Ok(3 + size)
                }
                DISC_FUNCTION => {
                    // Validate params and return type
                    let (param_count, mut pos) = read_varint(payload)?;
                    for _ in 0..param_count {
                        let param_len = validate_encoding(&payload[pos..], depth + 1)?;
                        pos += param_len;
                    }
                    let ret_len = validate_encoding(&payload[pos..], depth + 1)?;
                    pos += ret_len;
                    if pos != size {
                        return Err(DecodeError::SizeMismatch {
                            offset: 0,
                            claimed: size,
                            actual: pos,
                        });
                    }
                    Ok(3 + size)
                }
                DISC_SYMBOL => {
                    // Validate part count and parts
                    let (part_count, mut pos) = read_varint(payload)?;
                    for _ in 0..part_count {
                        let (_, part_len) = read_string(&payload[pos..])?;
                        pos += part_len;
                    }
                    if pos != size {
                        return Err(DecodeError::SizeMismatch {
                            offset: 0,
                            claimed: size,
                            actual: pos,
                        });
                    }
                    Ok(3 + size)
                }
                DISC_TYPEVAR => {
                    // Non-packed typevar: should have size=2 for u16
                    if size != 2 {
                        return Err(DecodeError::SizeMismatch {
                            offset: 0,
                            claimed: size,
                            actual: 2,
                        });
                    }
                    Ok(5) // 3 + 2
                }
                _ => Err(DecodeError::UnknownDiscriminant {
                    discriminant: disc,
                    offset: 0,
                }),
            }
        }
        CAT_PACKED => {
            // Packed types: single byte
            if (DISC_ARRAY_INT..=DISC_ARRAY_BYTES).contains(&disc) {
                Ok(1)
            } else if (DISC_MAP_BASE..=DISC_MAP_BASE + 24).contains(&disc) {
                Ok(1)
            } else {
                Err(DecodeError::UnknownDiscriminant {
                    discriminant: disc,
                    offset: 0,
                })
            }
        }
        _ => unreachable!("category is 0-3"),
    }
}

// ============================================================================
// Decoding
// ============================================================================

/// Decode bytes into a Type, with full validation.
///
/// Returns error if encoding is malformed.
/// Successfully decoded types are interned in the TypeManager.
///
/// # Examples
///
/// ```
/// # use melbi_core::types::{manager::TypeManager, encoding::{encode, decode}};
/// # use bumpalo::Bump;
/// let arena = Bump::new();
/// let mgr = TypeManager::new(&arena);
///
/// let original = mgr.map(mgr.int(), mgr.str());
/// let bytes = encode(original);
/// let decoded = decode(&bytes, &mgr).unwrap();
///
/// // Interning preserved: same pointer
/// assert!(core::ptr::eq(original, decoded));
/// ```
pub fn decode<'a>(
    bytes: &[u8],
    mgr: &'a crate::types::manager::TypeManager<'a>,
) -> Result<&'a Type<'a>, DecodeError> {
    decode_inner(bytes, mgr, 0).map(|(ty, _)| ty)
}

fn decode_inner<'a>(
    bytes: &[u8],
    mgr: &'a crate::types::manager::TypeManager<'a>,
    depth: usize,
) -> Result<(&'a Type<'a>, usize), DecodeError> {
    const MAX_DEPTH: usize = 1000;

    if depth > MAX_DEPTH {
        return Err(DecodeError::TooDeep { depth });
    }

    if bytes.is_empty() {
        return Err(DecodeError::Truncated {
            offset: 0,
            needed: 1,
        });
    }

    let disc = bytes[0] & 0x7F;
    let cat = disc >> 5;

    match cat {
        CAT_TYPEVAR => {
            // Packed typevar: [0-31]
            let id = disc as u16;
            Ok((mgr.type_var(id), 1))
        }
        CAT_UNITARY => {
            // Primitives: [32-36]
            match disc {
                DISC_INT => Ok((mgr.int(), 1)),
                DISC_FLOAT => Ok((mgr.float(), 1)),
                DISC_BOOL => Ok((mgr.bool(), 1)),
                DISC_STR => Ok((mgr.str(), 1)),
                DISC_BYTES => Ok((mgr.bytes(), 1)),
                _ => Err(DecodeError::UnknownDiscriminant {
                    discriminant: disc,
                    offset: 0,
                }),
            }
        }
        CAT_COMPOSITE => {
            // Composite types: [disc][size:u16][payload]
            if bytes.len() < 3 {
                return Err(DecodeError::Truncated {
                    offset: 0,
                    needed: 3,
                });
            }

            let size = read_u16_le(&bytes[1..3]) as usize;
            if bytes.len() < 3 + size {
                return Err(DecodeError::Truncated {
                    offset: bytes.len(),
                    needed: 3 + size,
                });
            }

            let payload = &bytes[3..3 + size];

            match disc {
                // [64][size:u16][elem]
                DISC_ARRAY => {
                    let (elem, elem_len) = decode_inner(payload, mgr, depth + 1)?;
                    if elem_len != size {
                        return Err(DecodeError::SizeMismatch {
                            offset: 0,
                            claimed: size,
                            actual: elem_len,
                        });
                    }
                    Ok((mgr.array(elem), 3 + size))
                }

                // [65][size:u16][key][val]
                DISC_MAP => {
                    let (key, key_len) = decode_inner(payload, mgr, depth + 1)?;
                    let (val, val_len) = decode_inner(&payload[key_len..], mgr, depth + 1)?;
                    if key_len + val_len != size {
                        return Err(DecodeError::SizeMismatch {
                            offset: 0,
                            claimed: size,
                            actual: key_len + val_len,
                        });
                    }
                    Ok((mgr.map(key, val), 3 + size))
                }

                // [66][size:u16][count:varint]([name_len:varint][name][type])*
                DISC_RECORD => {
                    let (field_count, mut pos) = read_varint(payload)?;
                    let mut fields = Vec::with_capacity(field_count);

                    for _ in 0..field_count {
                        let (name, name_len) = read_string(&payload[pos..])?;
                        pos += name_len;

                        let (ty, ty_len) = decode_inner(&payload[pos..], mgr, depth + 1)?;
                        pos += ty_len;

                        fields.push((name, ty));
                    }

                    if pos != size {
                        return Err(DecodeError::SizeMismatch {
                            offset: 0,
                            claimed: size,
                            actual: pos,
                        });
                    }

                    Ok((mgr.record(fields), 3 + size))
                }

                // [67][size:u16][param_count:varint]([param])*[ret]
                DISC_FUNCTION => {
                    let (param_count, mut pos) = read_varint(payload)?;
                    let mut params = Vec::with_capacity(param_count);

                    for _ in 0..param_count {
                        let (param, param_len) = decode_inner(&payload[pos..], mgr, depth + 1)?;
                        pos += param_len;
                        params.push(param);
                    }

                    let (ret, ret_len) = decode_inner(&payload[pos..], mgr, depth + 1)?;
                    pos += ret_len;

                    if pos != size {
                        return Err(DecodeError::SizeMismatch {
                            offset: 0,
                            claimed: size,
                            actual: pos,
                        });
                    }

                    Ok((mgr.function(&params, ret), 3 + size))
                }

                // [68][size:u16][part_count:varint]([part_len:varint][part])*
                DISC_SYMBOL => {
                    let (part_count, mut pos) = read_varint(payload)?;
                    let mut parts = Vec::with_capacity(part_count);

                    for _ in 0..part_count {
                        let (part, part_len) = read_string(&payload[pos..])?;
                        pos += part_len;
                        parts.push(part);
                    }

                    if pos != size {
                        return Err(DecodeError::SizeMismatch {
                            offset: 0,
                            claimed: size,
                            actual: pos,
                        });
                    }

                    Ok((mgr.symbol(parts), 3 + size))
                }

                // [69][size:u16][id_lo][id_hi]
                DISC_TYPEVAR => {
                    if size != 2 {
                        return Err(DecodeError::SizeMismatch {
                            offset: 0,
                            claimed: size,
                            actual: 2,
                        });
                    }
                    let id = read_u16_le(payload);
                    Ok((mgr.type_var(id), 5))
                }

                _ => Err(DecodeError::UnknownDiscriminant {
                    discriminant: disc,
                    offset: 0,
                }),
            }
        }
        CAT_PACKED => {
            // Packed arrays: [96-100]
            if (DISC_ARRAY_INT..=DISC_ARRAY_BYTES).contains(&disc) {
                let prim_idx = disc - DISC_ARRAY_INT;
                let elem = primitive_by_idx(prim_idx, mgr);
                Ok((mgr.array(elem), 1))
            }
            // Packed maps: [101-125]
            else if (DISC_MAP_BASE..=DISC_MAP_BASE + 24).contains(&disc) {
                let idx = disc - DISC_MAP_BASE;
                let key_idx = idx / 5;
                let val_idx = idx % 5;
                let key = primitive_by_idx(key_idx, mgr);
                let val = primitive_by_idx(val_idx, mgr);
                Ok((mgr.map(key, val), 1))
            } else {
                Err(DecodeError::UnknownDiscriminant {
                    discriminant: disc,
                    offset: 0,
                })
            }
        }
        _ => unreachable!("category is 0-3"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::manager::TypeManager;
    use bumpalo::Bump;

    // ============================================================================
    // Basic Round-trip Tests
    // ============================================================================

    #[test]
    fn test_primitives_round_trip() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let types = [mgr.int(), mgr.float(), mgr.bool(), mgr.str(), mgr.bytes()];

        for ty in &types {
            let bytes = encode(ty);
            let decoded = decode(&bytes, &mgr).unwrap();
            assert!(core::ptr::eq(*ty, decoded), "round-trip failed for {}", ty);
        }
    }

    #[test]
    fn test_packed_typevars() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Test packed range (0-31)
        for id in 0..=31 {
            let ty = mgr.type_var(id);
            let bytes = encode(ty);

            // Should be single byte
            assert_eq!(bytes.len(), 1);
            assert_eq!(bytes[0], id as u8);

            let decoded = decode(&bytes, &mgr).unwrap();
            assert!(core::ptr::eq(ty, decoded));
        }

        // Test non-packed (>= 32)
        let ty = mgr.type_var(100);
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 5); // disc + size + id

        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_packed_arrays() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let tests = [
            (mgr.array(mgr.int()), DISC_ARRAY_INT),
            (mgr.array(mgr.float()), DISC_ARRAY_FLOAT),
            (mgr.array(mgr.bool()), DISC_ARRAY_BOOL),
            (mgr.array(mgr.str()), DISC_ARRAY_STR),
            (mgr.array(mgr.bytes()), DISC_ARRAY_BYTES),
        ];

        for (ty, expected_disc) in &tests {
            let bytes = encode(ty);
            assert_eq!(bytes.len(), 1);
            assert_eq!(bytes[0], *expected_disc);

            let decoded = decode(&bytes, &mgr).unwrap();
            assert!(core::ptr::eq(*ty, decoded));
        }
    }

    #[test]
    fn test_non_packed_array() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Array[Array[Int]] - not packable
        let ty = mgr.array(mgr.array(mgr.int()));
        let bytes = encode(ty);

        // Should be: [DISC_ARRAY][size:u16][DISC_ARRAY_INT]
        assert!(bytes.len() > 1);
        assert_eq!(bytes[0], DISC_ARRAY);

        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_packed_maps() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Test all 25 combinations of primitive maps
        let prims = [mgr.int(), mgr.float(), mgr.bool(), mgr.str(), mgr.bytes()];

        for (key_idx, key) in prims.iter().enumerate() {
            for (val_idx, val) in prims.iter().enumerate() {
                let ty = mgr.map(*key, *val);
                let bytes = encode(ty);

                // Should be single byte
                assert_eq!(bytes.len(), 1);
                let expected_disc = DISC_MAP_BASE + (key_idx * 5 + val_idx) as u8;
                assert_eq!(bytes[0], expected_disc);

                let decoded = decode(&bytes, &mgr).unwrap();
                assert!(core::ptr::eq(ty, decoded));
            }
        }
    }

    #[test]
    fn test_non_packed_map() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Map[Array[Int], Int] - not packable
        let ty = mgr.map(mgr.array(mgr.int()), mgr.int());
        let bytes = encode(ty);

        assert!(bytes.len() > 1);
        assert_eq!(bytes[0], DISC_MAP);

        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_record_round_trip() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let fields = vec![
            ("name", mgr.str()),
            ("age", mgr.int()),
            ("active", mgr.bool()),
        ];

        let ty = mgr.record(fields);
        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();

        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_empty_record() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.record(vec![]);
        let bytes = encode(ty);

        // [DISC_RECORD][size:u16][count=0]
        assert_eq!(bytes[0], DISC_RECORD);

        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_function_round_trip() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.function(&[mgr.int(), mgr.str(), mgr.bool()], mgr.float());

        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();

        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_function_no_params() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.function(&[], mgr.int());
        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();

        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_symbol_round_trip() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.symbol(vec!["success", "error", "pending"]);
        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();

        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_complex_nested_type() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Function(
        //   Map[Str, Array[Int]],
        //   Record[result: Bool, count: Int]
        // ) => Symbol[success|error]
        let ty = mgr.function(
            &[
                mgr.map(mgr.str(), mgr.array(mgr.int())),
                mgr.record(vec![("result", mgr.bool()), ("count", mgr.int())]),
            ],
            mgr.symbol(vec!["success", "error"]),
        );

        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();

        assert!(core::ptr::eq(ty, decoded));
    }

    // ============================================================================
    // TypeView Navigation Tests
    // ============================================================================

    #[test]
    fn test_navigate_map_str_int() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.map(mgr.str(), mgr.int());
        let bytes = encode(ty);

        let view = TypeView::new(&bytes);

        assert_eq!(view.discriminant(), DISC_MAP);
        let (key, val) = view.as_map().expect("should be a map!");
        assert_eq!(key.discriminant(), DISC_STR);
        assert_eq!(val.discriminant(), DISC_INT);
    }

    #[test]
    fn test_typeview_category() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let tests = [
            (mgr.type_var(5), CAT_TYPEVAR),
            (mgr.int(), CAT_UNITARY),
            (mgr.array(mgr.array(mgr.int())), CAT_COMPOSITE),
            (mgr.array(mgr.int()), CAT_PACKED),
        ];

        for (ty, expected_cat) in &tests {
            let bytes = encode(ty);
            let view = TypeView::new(&bytes);
            assert_eq!(view.category(), *expected_cat);
        }
    }

    #[test]
    fn test_typeview_is_unitary() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Unitary types
        let unitary = [
            mgr.int(),
            mgr.type_var(10),
            mgr.array(mgr.int()),            // packed
            mgr.map(mgr.int(), mgr.float()), // packed
        ];

        for ty in &unitary {
            let bytes = encode(ty);
            let view = TypeView::new(&bytes);
            assert!(view.is_unitary(), "{} should be unitary", ty);
        }

        // Non-unitary (composite)
        let composite = [
            mgr.array(mgr.array(mgr.int())),
            mgr.record(vec![("x", mgr.int())]),
            mgr.function(&[mgr.int()], mgr.int()),
        ];

        for ty in &composite {
            let bytes = encode(ty);
            let view = TypeView::new(&bytes);
            assert!(!view.is_unitary(), "{} should not be unitary", ty);
        }
    }

    #[test]
    fn test_typeview_as_array() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Non-packed array
        let ty = mgr.array(mgr.array(mgr.int()));
        let bytes = encode(ty);
        let view = TypeView::new(&bytes);

        let elem_view = view.as_array().unwrap();
        assert_eq!(elem_view.discriminant(), DISC_ARRAY_INT);
    }

    #[test]
    fn test_typeview_as_map() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Non-packed map
        let ty = mgr.map(mgr.array(mgr.int()), mgr.str());
        let bytes = encode(ty);
        let view = TypeView::new(&bytes);

        let (key_view, val_view) = view.as_map().unwrap();
        assert_eq!(key_view.discriminant(), DISC_ARRAY_INT);
        assert_eq!(val_view.discriminant(), DISC_STR);
    }

    #[test]
    fn test_typeview_as_record() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.record(vec![
            ("name", mgr.str()),
            ("age", mgr.int()),
            ("active", mgr.bool()),
        ]);

        let bytes = encode(ty);
        let view = TypeView::new(&bytes);

        // Sorted order:
        let mut iter = view.as_record().unwrap();
        assert_eq!(iter.len(), 3);

        {
            let (name, ty) = iter.next().unwrap().unwrap();
            assert_eq!(name, "active");
            assert_eq!(ty.discriminant(), DISC_BOOL);
        }

        {
            let (name, ty) = iter.next().unwrap().unwrap();
            assert_eq!(name, "age");
            assert_eq!(ty.discriminant(), DISC_INT);
        }

        {
            let (name, ty) = iter.next().unwrap().unwrap();
            assert_eq!(name, "name");
            assert_eq!(ty.discriminant(), DISC_STR);
        }

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_typeview_as_function() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.function(&[mgr.int(), mgr.str(), mgr.bool()], mgr.float());

        let bytes = encode(ty);
        let view = TypeView::new(&bytes);

        let func_view = view.as_function().unwrap();

        let params: Vec<_> = func_view.params().collect();
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].as_ref().unwrap().discriminant(), DISC_INT);
        assert_eq!(params[1].as_ref().unwrap().discriminant(), DISC_STR);
        assert_eq!(params[2].as_ref().unwrap().discriminant(), DISC_BOOL);

        let ret = func_view.return_type().unwrap();
        assert_eq!(ret.discriminant(), DISC_FLOAT);
    }

    #[test]
    fn test_typeview_as_symbol() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.symbol(vec!["success", "error", "pending"]);
        let bytes = encode(ty);
        let view = TypeView::new(&bytes);

        let parts: Vec<_> = view.as_symbol().unwrap().collect();
        assert_eq!(parts.len(), 3);
        // TypeManager sorts the parts.
        assert_eq!(parts[0].as_ref().unwrap(), &"error");
        assert_eq!(parts[1].as_ref().unwrap(), &"pending");
        assert_eq!(parts[2].as_ref().unwrap(), &"success");
    }

    #[test]
    fn test_typeview_as_typevar() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Packed
        let ty = mgr.type_var(15);
        let bytes = encode(ty);
        let view = TypeView::new(&bytes);
        assert_eq!(view.as_typevar().unwrap(), 15);

        // Non-packed
        let ty = mgr.type_var(100);
        let bytes = encode(ty);
        let view = TypeView::new(&bytes);
        assert_eq!(view.as_typevar().unwrap(), 100);
    }

    // ============================================================================
    // Error Handling Tests
    // ============================================================================

    #[test]
    fn test_decode_truncated() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Empty buffer
        let result = decode(&[], &mgr);
        assert!(matches!(result, Err(DecodeError::Truncated { .. })));

        // Composite type with incomplete size field
        let bytes = vec![DISC_ARRAY, 5]; // Missing second byte of size
        let result = decode(&bytes, &mgr);
        assert!(matches!(result, Err(DecodeError::Truncated { .. })));

        // Size field larger than buffer
        let bytes = vec![DISC_ARRAY, 10, 0]; // Claims 10 bytes but buffer ends
        let result = decode(&bytes, &mgr);
        assert!(matches!(result, Err(DecodeError::Truncated { .. })));
    }

    #[test]
    fn test_decode_unknown_discriminant() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Unknown unitary type
        let bytes = vec![37]; // Reserved slot in unitary range
        let result = decode(&bytes, &mgr);
        assert!(matches!(
            result,
            Err(DecodeError::UnknownDiscriminant { .. })
        ));

        // Unknown composite type
        let bytes = vec![70, 0, 0]; // Reserved slot in composite range
        let result = decode(&bytes, &mgr);
        assert!(matches!(
            result,
            Err(DecodeError::UnknownDiscriminant { .. })
        ));
    }

    #[test]
    fn test_decode_size_mismatch() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Array with wrong size field
        let mut bytes = vec![DISC_ARRAY, 5, 0]; // Claims 5 bytes
        bytes.push(DISC_INT); // But int is only 1 byte
        bytes.extend_from_slice(&[0, 0, 0, 0]); // Pad to claimed size

        let result = decode(&bytes, &mgr);
        assert!(matches!(result, Err(DecodeError::SizeMismatch { .. })));
    }

    #[test]
    fn test_decode_invalid_utf8() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Record with invalid UTF-8 field name
        let mut bytes = vec![DISC_RECORD, 0, 0]; // Will backpatch size
        let start = bytes.len();
        bytes.push(1); // 1 field
        bytes.push(3); // field name length = 3
        bytes.extend_from_slice(&[0xFF, 0xFE, 0xFD]); // Invalid UTF-8
        bytes.push(DISC_INT); // field type

        // Backpatch size
        let size = bytes.len() - start;
        bytes[1] = (size & 0xFF) as u8;
        bytes[2] = ((size >> 8) & 0xFF) as u8;

        let result = decode(&bytes, &mgr);
        assert!(matches!(result, Err(DecodeError::InvalidUtf8 { .. })));
    }

    #[test]
    fn test_validation_success() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.function(
            &[mgr.map(mgr.str(), mgr.array(mgr.int()))],
            mgr.record(vec![("result", mgr.bool())]),
        );

        let bytes = encode(ty);
        let result = TypeView::validated(&bytes);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_failure() {
        // Malformed encoding: size mismatch
        let mut bytes = vec![DISC_ARRAY, 10, 0]; // Claims 10 bytes
        bytes.push(DISC_INT); // But int is only 1 byte
        bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0]); // Pad to claimed size

        let result = TypeView::validated(&bytes);
        assert!(matches!(result, Err(DecodeError::SizeMismatch { .. })));
    }

    // ============================================================================
    // Property Tests
    // ============================================================================

    #[test]
    fn test_encode_deterministic() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.function(&[mgr.map(mgr.str(), mgr.int())], mgr.array(mgr.bool()));

        // Encode multiple times
        let bytes1 = encode(ty);
        let bytes2 = encode(ty);
        let bytes3 = encode(ty);

        assert_eq!(bytes1.as_slice(), bytes2.as_slice());
        assert_eq!(bytes2.as_slice(), bytes3.as_slice());
    }

    #[test]
    fn test_structural_equality_via_bytes() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Create same type twice (different pointers due to fresh construction)
        let ty1 = mgr.map(mgr.int(), mgr.str());
        let ty2 = mgr.map(mgr.int(), mgr.str());

        // Should be interned to same pointer
        assert!(core::ptr::eq(ty1, ty2));

        // And encode to same bytes
        let bytes1 = encode(ty1);
        let bytes2 = encode(ty2);
        assert_eq!(bytes1.as_slice(), bytes2.as_slice());
    }

    #[test]
    fn test_different_types_different_bytes() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty1 = mgr.map(mgr.int(), mgr.str());
        let ty2 = mgr.map(mgr.str(), mgr.int()); // Swapped

        let bytes1 = encode(ty1);
        let bytes2 = encode(ty2);
        assert_ne!(bytes1.as_slice(), bytes2.as_slice());
    }

    // ============================================================================
    // Edge Cases
    // ============================================================================

    #[test]
    fn test_deeply_nested_type() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Create deeply nested array: Array[Array[Array[...]]]
        let mut ty = mgr.int();
        for _ in 0..100 {
            ty = mgr.array(ty);
        }

        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_large_record() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Create record with many fields
        let fields: Vec<_> = (0..100)
            .map(|i| {
                let name = arena.alloc_str(&format!("field_{}", i));
                (name as &str, mgr.int())
            })
            .collect();

        let ty = mgr.record(fields);
        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_unicode_field_names() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let fields = vec![("名前", mgr.str()), ("年齢", mgr.int()), ("🎉", mgr.bool())];

        let ty = mgr.record(fields);
        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));

        // Verify names preserved
        let view = TypeView::new(&bytes);
        let field_names: Vec<_> = view.as_record().unwrap().map(|r| r.unwrap().0).collect();

        assert_eq!(field_names, vec!["名前", "年齢", "🎉"]);
    }

    #[test]
    fn test_empty_containers() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Empty record
        let ty = mgr.record(vec![]);
        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));

        // Function with no params
        let ty = mgr.function(&[], mgr.int());
        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));

        // Symbol with no parts (unusual but valid)
        let ty = mgr.symbol(vec![]);
        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_accept_non_canonical_encoding() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Manually create non-canonical encoding: [DISC_ARRAY][size][DISC_INT]
        // This should be [DISC_ARRAY_INT] but we accept the longer form
        let bytes = vec![DISC_ARRAY, 1, 0, DISC_INT];

        let decoded = decode(&bytes, &mgr).unwrap();
        let expected = mgr.array(mgr.int());
        assert!(core::ptr::eq(decoded, expected));
    }
}
