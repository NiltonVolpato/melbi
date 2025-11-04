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
//! ## Format Design Principles
//!
//! - **Singleton before array**: When a type contains both a singleton and array,
//!   the singleton is encoded first for O(1) access (e.g., Function return type).
//! - **Count prefixes sequence**: Array counts always immediately precede their elements:
//!   `[varint:count][element_1][element_2]...[element_n]`
//!
//! Example: Function type encoding:
//! ```text
//! [DISC_FUNCTION][size_lo][size_hi][return_type][varint:param_count][param_1][param_2]...
//! ```
//!
//! See the design document for full specification.

use crate::{Type, Vec, types::type_traits::TypeTag};
use alloc::sync::Arc;
use core::fmt;
use smallvec::SmallVec;

// ============================================================================
// TypeStorage Trait
// ============================================================================

/// Abstraction over byte storage for encoded types.
///
/// This trait enables `EncodedType<S>` to work with both borrowed and owned
/// byte storage, supporting different use cases:
///
/// - `&[u8]`: Zero-copy, borrowed from arena or buffer (Copy)
/// - `Arc<[u8]>`: Owned, shareable across threads, can outlive source (Clone only)
///
/// All implementations must be `Clone`. `EncodedType<S>` will be `Copy` when `S: Copy`.
pub trait TypeStorage: Clone {
    /// Returns a reference to the underlying bytes.
    fn as_bytes(&self) -> &[u8];
}

impl TypeStorage for &[u8] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self
    }
}

impl TypeStorage for Arc<[u8]> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        &self[..]
    }
}

// ============================================================================
// Wire Format Encoding
// ============================================================================
//
// IMPORTANT: This encoding is for in-memory representation only, NOT for
// data exchange or persistent storage. The format can change freely between
// versions without compatibility concerns.
//
// Wire Format Layout:
// - Bytes 1-5:    Unit types (Int=1, Float=2, Bool=3, Str=4, Bytes=5)
// - Bytes 64-74:  Composite types (TypeVar=64, Array=70, Map=71, Record=72, Function=73, Symbol=74)
// - Bytes 96-127: Packed optimizations (TypeVar packed: 96-127 for IDs 0-31)
//
// Composite types are encoded as: [wire_byte][size_lo][size_hi][...payload...]
//
// Adding a new type: Update TypeTag enum in type_traits.rs and WireTag implementation below.

/// Instruction for encoding: which format to prefer (INPUT)
#[derive(Debug, Clone, Copy)]
enum WireEncoding {
    Standard,                    // Use standard format (0-10) with payload
    PackedTypeVar(u16),          // Try packed TypeVar (64-95)
    PackedArray(TypeTag),        // Try packed Array (96-100)
    PackedMap(TypeTag, TypeTag), // Try packed Map (101-125)
}

/// Result after decoding: where to get the type data (OUTPUT)
#[derive(Debug, Clone, Copy)]
enum PayloadLocation {
    LongFormat,                              // Read payload from byte stream
    PackedTypeVar(u16),                      // TypeVar ID embedded in wire byte
    PackedArray(&'static [u8]),              // Element type bytes (static)
    PackedMap(&'static [u8], &'static [u8]), // Key and value type bytes (static)
}

#[derive(Debug, Clone, Copy)]
struct WireTag {
    type_tag: TypeTag,
    payload: PayloadLocation,
}

impl WireTag {
    /// Get static bytes for a primitive type tag
    const fn encoded_bytes(tag: TypeTag) -> Option<&'static [u8]> {
        // Static bytes representing each primitive type wire tag
        // These must use wire tag values to ensure they match the actual wire format
        static INT_BYTES: [u8; 1] = [TypeTag::Int as u8];
        static FLOAT_BYTES: [u8; 1] = [TypeTag::Float as u8];
        static BOOL_BYTES: [u8; 1] = [TypeTag::Bool as u8];
        static STR_BYTES: [u8; 1] = [TypeTag::Str as u8];
        static BYTES_BYTES: [u8; 1] = [TypeTag::Bytes as u8];
        match tag {
            TypeTag::Int => Some(&INT_BYTES),
            TypeTag::Float => Some(&FLOAT_BYTES),
            TypeTag::Bool => Some(&BOOL_BYTES),
            TypeTag::Str => Some(&STR_BYTES),
            TypeTag::Bytes => Some(&BYTES_BYTES),
            _ => None,
        }
    }

    /// Convert TypeTag to canonical wire byte (standard format, matches TypeTag discriminant 1:1)
    const fn to_wire_byte(type_tag: TypeTag) -> u8 {
        type_tag as u8
    }

    /// Convert primitive TypeTag to index (0-4) for packed encoding
    fn type_tag_to_primitive_index(tag: TypeTag) -> Option<u8> {
        match tag {
            TypeTag::Int => Some(0),
            TypeTag::Float => Some(1),
            TypeTag::Bool => Some(2),
            TypeTag::Str => Some(3),
            TypeTag::Bytes => Some(4),
            _ => None,
        }
    }

    /// Decode wire byte into WireTag
    fn from_byte(byte: u8) -> Result<Self, DecodeError> {
        match byte {
            // Standard wire tags (0-10): Match TypeTag discriminants exactly
            0 => Ok(WireTag {
                type_tag: TypeTag::TypeVar,
                payload: PayloadLocation::LongFormat,
            }),
            1 => Ok(WireTag {
                type_tag: TypeTag::Int,
                payload: PayloadLocation::LongFormat,
            }),
            2 => Ok(WireTag {
                type_tag: TypeTag::Float,
                payload: PayloadLocation::LongFormat,
            }),
            3 => Ok(WireTag {
                type_tag: TypeTag::Bool,
                payload: PayloadLocation::LongFormat,
            }),
            4 => Ok(WireTag {
                type_tag: TypeTag::Str,
                payload: PayloadLocation::LongFormat,
            }),
            5 => Ok(WireTag {
                type_tag: TypeTag::Bytes,
                payload: PayloadLocation::LongFormat,
            }),
            6 => Ok(WireTag {
                type_tag: TypeTag::Array,
                payload: PayloadLocation::LongFormat,
            }),
            7 => Ok(WireTag {
                type_tag: TypeTag::Map,
                payload: PayloadLocation::LongFormat,
            }),
            8 => Ok(WireTag {
                type_tag: TypeTag::Record,
                payload: PayloadLocation::LongFormat,
            }),
            9 => Ok(WireTag {
                type_tag: TypeTag::Function,
                payload: PayloadLocation::LongFormat,
            }),
            10 => Ok(WireTag {
                type_tag: TypeTag::Symbol,
                payload: PayloadLocation::LongFormat,
            }),

            // Reserved (11-63): For future language types
            11..=63 => Err(DecodeError::UnknownDiscriminant {
                discriminant: byte,
                offset: 0,
            }),

            // Packed TypeVar (64-95): IDs 0-31
            64..=95 => {
                let id = (byte - 64) as u16;
                Ok(WireTag {
                    type_tag: TypeTag::TypeVar,
                    payload: PayloadLocation::PackedTypeVar(id),
                })
            }

            // Packed Array (96-100): Array[Primitive]
            96 => Ok(WireTag {
                type_tag: TypeTag::Array,
                payload: PayloadLocation::PackedArray(&INT_BYTES),
            }),
            97 => Ok(WireTag {
                type_tag: TypeTag::Array,
                payload: PayloadLocation::PackedArray(&FLOAT_BYTES),
            }),
            98 => Ok(WireTag {
                type_tag: TypeTag::Array,
                payload: PayloadLocation::PackedArray(&BOOL_BYTES),
            }),
            99 => Ok(WireTag {
                type_tag: TypeTag::Array,
                payload: PayloadLocation::PackedArray(&STR_BYTES),
            }),
            100 => Ok(WireTag {
                type_tag: TypeTag::Array,
                payload: PayloadLocation::PackedArray(&BYTES_BYTES),
            }),

            // Packed Map (101-125): Map[Primitive, Primitive]
            101..=125 => {
                let offset = byte - 101;
                let key_idx = offset / 5;
                let val_idx = offset % 5;

                let key_tag = primitive_index_to_type_tag(key_idx)?;
                let val_tag = primitive_index_to_type_tag(val_idx)?;

                Ok(WireTag {
                    type_tag: TypeTag::Map,
                    payload: PayloadLocation::PackedMap(
                        Self::encoded_bytes(key_tag),
                        Self::encoded_bytes(val_tag),
                    ),
                })
            }

            // Reserved/Invalid (126-255): MSB set or future use
            _ => Err(DecodeError::UnknownDiscriminant {
                discriminant: byte,
                offset: 0,
            }),
        }
    }

    /// Create WireTag for encoding (tries to use packed format when possible)
    fn for_encoding(type_tag: TypeTag, encoding: WireEncoding) -> Self {
        match (type_tag, encoding) {
            // Try packed TypeVar
            (TypeTag::TypeVar, WireEncoding::PackedTypeVar(id)) if id <= 31 => WireTag {
                type_tag: TypeTag::TypeVar,
                payload: PayloadLocation::PackedTypeVar(id),
            },

            // Try packed Array
            (TypeTag::Array, WireEncoding::PackedArray(elem)) if Self::is_primitive(elem) => {
                WireTag {
                    type_tag: TypeTag::Array,
                    payload: PayloadLocation::PackedArray(Self::encoded_bytes(elem)),
                }
            }

            // Try packed Map
            (TypeTag::Map, WireEncoding::PackedMap(key, val))
                if Self::is_primitive(key) && Self::is_primitive(val) =>
            {
                WireTag {
                    type_tag: TypeTag::Map,
                    payload: PayloadLocation::PackedMap(
                        Self::encoded_bytes(key),
                        Self::encoded_bytes(val),
                    ),
                }
            }

            // Fall back to standard format for all other cases
            _ => WireTag {
                type_tag,
                payload: PayloadLocation::LongFormat,
            },
        }
    }

    /// Encode to wire byte
    fn to_byte(&self) -> u8 {
        match self.payload {
            PayloadLocation::PackedTypeVar(id) => 64 + (id as u8),

            PayloadLocation::PackedArray(elem_bytes) => {
                // Reverse lookup: which primitive?
                let elem_byte = elem_bytes[0];
                let elem_tag = TypeTag::try_from(elem_byte).expect("Invalid primitive byte");
                let idx = Self::type_tag_to_primitive_index(elem_tag).expect("Not a primitive");
                96 + idx
            }

            PayloadLocation::PackedMap(key_bytes, val_bytes) => {
                let key_byte = key_bytes[0];
                let val_byte = val_bytes[0];
                let key_tag = TypeTag::try_from(key_byte).expect("Invalid primitive byte");
                let val_tag = TypeTag::try_from(val_byte).expect("Invalid primitive byte");
                let key_idx = Self::type_tag_to_primitive_index(key_tag).expect("Not a primitive");
                let val_idx = Self::type_tag_to_primitive_index(val_tag).expect("Not a primitive");
                101 + key_idx * 5 + val_idx
            }

            PayloadLocation::LongFormat => Self::to_wire_byte(self.type_tag),
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert primitive index (0-4) to TypeTag
fn primitive_index_to_type_tag(idx: u8) -> Result<TypeTag, DecodeError> {
    match idx {
        0 => Ok(TypeTag::Int),
        1 => Ok(TypeTag::Float),
        2 => Ok(TypeTag::Bool),
        3 => Ok(TypeTag::Str),
        4 => Ok(TypeTag::Bytes),
        _ => Err(DecodeError::UnknownDiscriminant {
            discriminant: idx,
            offset: 0,
        }),
    }
}

/// Get TypeTag from Type if it's a primitive (for packed encoding)
fn type_to_type_tag(ty: &Type) -> Option<TypeTag> {
    match ty {
        Type::Int => Some(TypeTag::Int),
        Type::Float => Some(TypeTag::Float),
        Type::Bool => Some(TypeTag::Bool),
        Type::Str => Some(TypeTag::Str),
        Type::Bytes => Some(TypeTag::Bytes),
        _ => None,
    }
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Truncated {
        offset: usize,
        needed: usize,
    },
    SizeMismatch {
        offset: usize,
        claimed: usize,
        actual: usize,
    },
    UnknownDiscriminant {
        discriminant: u8,
        offset: usize,
    },
    InvalidUtf8 {
        offset: usize,
    },
    InvalidVarint {
        offset: usize,
    },
    TooDeep {
        depth: usize,
    },
    TrailingBytes {
        offset: usize,
        remaining: usize,
    },
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
            DecodeError::TrailingBytes { offset, remaining } => {
                write!(
                    f,
                    "trailing bytes at offset {}: {} bytes remaining",
                    offset, remaining
                )
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

// ============================================================================
// Helper Functions
// ============================================================================

fn write_varint(buf: &mut OutputType, mut n: usize) {
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

fn write_string(buf: &mut OutputType, s: &str) {
    write_varint(buf, s.len());
    buf.extend_from_slice(s.as_bytes());
}

fn read_varint(bytes: &[u8]) -> Result<(usize, usize), DecodeError> {
    if bytes.is_empty() {
        return Err(DecodeError::Truncated {
            offset: 0,
            needed: 1,
        });
    }

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

// SAFETY: Strings are only written via `write_string()` which takes `&str`,
// guaranteeing valid UTF-8. This encoding format is for in-memory use only
// with trusted sources (we control both encode and decode).
fn read_string(bytes: &[u8]) -> Result<(&str, usize), DecodeError> {
    let (len, varint_len) = read_varint(bytes)?;

    if bytes.len() < varint_len + len {
        return Err(DecodeError::Truncated {
            offset: bytes.len(),
            needed: varint_len + len,
        });
    }

    let str_bytes = &bytes[varint_len..varint_len + len];
    let s = unsafe { core::str::from_utf8_unchecked(str_bytes) };

    Ok((s, varint_len + len))
}

#[inline]
fn read_u16_le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

#[inline]
fn write_u16_le(buf: &mut OutputType, val: u16) {
    buf.extend_from_slice(&val.to_le_bytes());
}

/// Validates that a composite type's size field matches the actual consumed bytes.
/// Only validates for non-packed composite types (those with 3-byte headers).
#[inline]
fn validate_composite_size<S: TypeStorage>(
    view: EncodedType<S>,
    actual_size: usize,
) -> Result<(), DecodeError> {
    let bytes = view.bytes();
    if bytes.len() < 3 {
        return Err(DecodeError::Truncated {
            offset: 0,
            needed: 3,
        });
    }
    let claimed_size = read_u16_le(&bytes[1..3]) as usize;
    if actual_size != claimed_size {
        return Err(DecodeError::SizeMismatch {
            offset: 0,
            claimed: claimed_size,
            actual: actual_size,
        });
    }
    Ok(())
}

// ============================================================================
// Encoding
// ============================================================================

pub type OutputType = SmallVec<[u8; 16]>;

/// Encodes a composite type with the standard 3-byte header: [disc][size_lo][size_hi][payload]
/// The payload is encoded by the provided closure.
#[inline]
fn encode_composite<F>(buf: &mut OutputType, disc: u8, encode_payload: F)
where
    F: FnOnce(&mut OutputType),
{
    let start = buf.len();
    buf.push(disc);
    buf.push(0); // placeholder for size_lo
    buf.push(0); // placeholder for size_hi
    encode_payload(buf);
    let size = buf.len() - start - 3;
    buf[start + 1] = (size & 0xFF) as u8;
    buf[start + 2] = ((size >> 8) & 0xFF) as u8;
}

pub fn encode(ty: &Type) -> OutputType {
    let mut buf = OutputType::new();
    encode_inner(ty, &mut buf);
    buf
}

fn encode_inner(ty: &Type, buf: &mut OutputType) {
    match ty {
        Type::TypeVar(id) => {
            // Try packed format for TypeVar IDs 0-31
            let tag = WireTag::for_encoding(TypeTag::TypeVar, WireEncoding::PackedTypeVar(*id));
            match tag.payload {
                PayloadLocation::PackedTypeVar(_) => {
                    // Packed format: single byte
                    buf.push(tag.to_byte());
                }
                PayloadLocation::LongFormat => {
                    // Long format: [wire_byte][size_lo][size_hi][id_lo][id_hi]
                    encode_composite(buf, tag.to_byte(), |buf| {
                        write_u16_le(buf, *id);
                    });
                }
                _ => unreachable!(),
            }
        }

        Type::Int => buf.push(WireTag::to_wire_byte(TypeTag::Int)),
        Type::Float => buf.push(WireTag::to_wire_byte(TypeTag::Float)),
        Type::Bool => buf.push(WireTag::to_wire_byte(TypeTag::Bool)),
        Type::Str => buf.push(WireTag::to_wire_byte(TypeTag::Str)),
        Type::Bytes => buf.push(WireTag::to_wire_byte(TypeTag::Bytes)),

        Type::Array(elem) => {
            // Try packed format for Array[Primitive]
            let elem_tag = type_to_type_tag(elem);
            let encoding = elem_tag
                .map(WireEncoding::PackedArray)
                .unwrap_or(WireEncoding::Standard);
            let tag = WireTag::for_encoding(TypeTag::Array, encoding);

            match tag.payload {
                PayloadLocation::PackedArray(_) => {
                    // Packed format: single byte
                    buf.push(tag.to_byte());
                }
                PayloadLocation::LongFormat => {
                    // Long format: [wire_byte][size_lo][size_hi][elem]
                    encode_composite(buf, tag.to_byte(), |buf| {
                        encode_inner(elem, buf);
                    });
                }
                _ => unreachable!(),
            }
        }

        Type::Map(key, val) => {
            // Try packed format for Map[Primitive, Primitive]
            let key_tag = type_to_type_tag(key);
            let val_tag = type_to_type_tag(val);
            let encoding = match (key_tag, val_tag) {
                (Some(k), Some(v)) => WireEncoding::PackedMap(k, v),
                _ => WireEncoding::Standard,
            };
            let tag = WireTag::for_encoding(TypeTag::Map, encoding);

            match tag.payload {
                PayloadLocation::PackedMap(_, _) => {
                    // Packed format: single byte
                    buf.push(tag.to_byte());
                }
                PayloadLocation::LongFormat => {
                    // Long format: [wire_byte][size_lo][size_hi][key][val]
                    encode_composite(buf, tag.to_byte(), |buf| {
                        encode_inner(key, buf);
                        encode_inner(val, buf);
                    });
                }
                _ => unreachable!(),
            }
        }

        Type::Record(fields) => {
            let tag = WireTag::for_encoding(TypeTag::Record, WireEncoding::Standard);
            encode_composite(buf, tag.to_byte(), |buf| {
                write_varint(buf, fields.len());
                for (name, ty) in fields.iter() {
                    write_string(buf, name);
                    encode_inner(ty, buf);
                }
            });
        }

        Type::Function { params, ret } => {
            let tag = WireTag::for_encoding(TypeTag::Function, WireEncoding::Standard);
            encode_composite(buf, tag.to_byte(), |buf| {
                encode_inner(ret, buf); // return type FIRST
                write_varint(buf, params.len()); // then count
                for param in params.iter() {
                    encode_inner(param, buf); // then params
                }
            });
        }

        Type::Symbol(parts) => {
            let tag = WireTag::for_encoding(TypeTag::Symbol, WireEncoding::Standard);
            encode_composite(buf, tag.to_byte(), |buf| {
                write_varint(buf, parts.len());
                for part in parts.iter() {
                    write_string(buf, part);
                }
            });
        }
    }
}

// ============================================================================
// EncodedType
// ============================================================================

/// An encoded type with flexible storage.
///
/// This type wraps byte storage (borrowed or owned) and provides zero-copy
/// navigation through the encoded type structure.
///
/// # Type Parameters
///
/// - `S`: Storage type implementing `TypeStorage` (e.g., `&[u8]`, `Arc<[u8]>`)
///
/// # Copy Semantics
///
/// `EncodedType<S>` is `Copy` when `S: Copy` (e.g., `&[u8]`), enabling implicit
/// copying like the arena-allocated types. For `Arc<[u8]>`, only `Clone` is available.
#[derive(Debug, Clone)]
pub struct EncodedType<S: TypeStorage> {
    storage: S,
}

// Manually implement Copy only when S is Copy
impl<S: TypeStorage + Copy> Copy for EncodedType<S> {}

// Implement equality via byte comparison
impl<S: TypeStorage> PartialEq for EncodedType<S> {
    fn eq(&self, other: &Self) -> bool {
        self.storage.as_bytes() == other.storage.as_bytes()
    }
}

impl<S: TypeStorage> Eq for EncodedType<S> {}

impl<S: TypeStorage> EncodedType<S> {
    #[inline]
    pub fn new(storage: S) -> Self {
        EncodedType { storage }
    }

    /// Get the underlying bytes
    #[inline]
    fn bytes(&self) -> &[u8] {
        self.storage.as_bytes()
    }
}

impl<S: TypeStorage> EncodedType<S> {
    #[inline]
    fn encoded_len(&self) -> usize {
        let bytes = self.bytes();
        if bytes.is_empty() {
            return 1; // Invalid, but return something
        }

        // Decode wire tag to check type and payload location
        match WireTag::from_byte(bytes[0]) {
            Ok(tag) => {
                // Primitives are always 1 byte (no payload)
                if matches!(
                    tag.type_tag,
                    TypeTag::Int | TypeTag::Float | TypeTag::Bool | TypeTag::Str | TypeTag::Bytes
                ) {
                    return 1;
                }

                // Packed types are always 1 byte
                if !matches!(tag.payload, PayloadLocation::LongFormat) {
                    return 1;
                }

                // Long format composite types: 3 + size
                if bytes.len() < 3 {
                    1 // Truncated, but return something
                } else {
                    3 + read_u16_le(&bytes[1..3]) as usize
                }
            }
            _ => 1, // Invalid: 1 byte
        }
    }

    #[inline]
    fn payload(&self) -> &[u8] {
        let bytes = self.bytes();
        if bytes.is_empty() {
            return &[];
        }

        // Check if this type has a payload section (LongFormat only)
        match WireTag::from_byte(bytes[0]) {
            Ok(tag) if matches!(tag.payload, PayloadLocation::LongFormat) => {
                // Composite types need at least 3 bytes: [disc][size_lo][size_hi]
                if bytes.len() < 3 {
                    return &[]; // Truncated
                }

                let size = read_u16_le(&bytes[1..3]) as usize;

                // Check bounds before slicing
                if bytes.len() < 3 + size {
                    return &[]; // Truncated
                }

                &bytes[3..3 + size]
            }
            _ => &[], // Packed types have no payload
        }
    }

    /// If this is a type variable, return its ID (works for packed and unpacked)
    pub fn as_typevar(self) -> Option<u16> {
        let bytes = self.bytes();
        if bytes.is_empty() {
            return None;
        }

        // Check wire tag
        let wire_tag = WireTag::from_byte(bytes[0]).ok()?;
        if wire_tag.type_tag != TypeTag::TypeVar {
            return None;
        }

        // Check for packed payload
        if let PayloadLocation::PackedTypeVar(id) = wire_tag.payload {
            return Some(id);
        }

        // Long format - read from payload
        let payload = self.payload();
        if payload.len() < 2 {
            return None;
        }
        Some(read_u16_le(payload))
    }
}

// Methods specific to EncodedType<&'a [u8]> that return iterators with lifetime 'a
impl<'a> EncodedType<&'a [u8]> {
    /// If this is an array, return element type view
    pub fn as_array(self) -> Option<EncodedType<&'a [u8]>> {
        let bytes = self.storage;
        if bytes.is_empty() {
            return None;
        }

        // Check wire tag
        let wire_tag = WireTag::from_byte(bytes[0]).ok()?;
        if wire_tag.type_tag != TypeTag::Array {
            return None;
        }

        // Handle packed arrays - return static element type bytes
        if let PayloadLocation::PackedArray(elem_bytes) = wire_tag.payload {
            return Some(EncodedType::new(elem_bytes));
        }

        // Long format - extract payload with lifetime 'a
        if bytes.len() < 3 {
            return None;
        }
        let size = read_u16_le(&bytes[1..3]) as usize;
        if bytes.len() < 3 + size {
            return None;
        }
        let payload: &'a [u8] = &bytes[3..3 + size];
        Some(EncodedType::new(payload))
    }

    /// If this is a map, return (key, value) views
    pub fn as_map(self) -> Option<(EncodedType<&'a [u8]>, EncodedType<&'a [u8]>)> {
        let bytes = self.storage;
        if bytes.is_empty() {
            return None;
        }

        // Check wire tag
        let wire_tag = WireTag::from_byte(bytes[0]).ok()?;
        if wire_tag.type_tag != TypeTag::Map {
            return None;
        }

        // Handle packed maps - return static key and value type bytes
        if let PayloadLocation::PackedMap(key_bytes, val_bytes) = wire_tag.payload {
            return Some((EncodedType::new(key_bytes), EncodedType::new(val_bytes)));
        }

        // Long format - extract payload with lifetime 'a
        if bytes.len() < 3 {
            return None;
        }
        let size = read_u16_le(&bytes[1..3]) as usize;
        if bytes.len() < 3 + size {
            return None;
        }
        let payload: &'a [u8] = &bytes[3..3 + size];

        // Check payload is not empty
        if payload.is_empty() {
            return None;
        }

        let key = EncodedType::new(payload);
        let key_len = key.encoded_len();

        // Check we have enough bytes for value
        if key_len > payload.len() {
            return None;
        }

        let val = EncodedType::new(&payload[key_len..]);
        Some((key, val))
    }

    pub fn as_record(self) -> Option<RecordIter<'a>> {
        let bytes = self.storage;
        if bytes.is_empty() {
            return None;
        }

        // Check wire tag
        let wire_tag = WireTag::from_byte(bytes[0]).ok()?;
        if wire_tag.type_tag != TypeTag::Record {
            return None;
        }

        // Extract payload with lifetime 'a from storage
        // Composite types have: [disc][size_lo][size_hi][...payload...]
        if bytes.len() < 3 {
            return None;
        }
        let size = read_u16_le(&bytes[1..3]) as usize;
        if bytes.len() < 3 + size {
            return None;
        }
        let payload: &'a [u8] = &bytes[3..3 + size];
        RecordIter::new(payload).ok()
    }

    pub fn as_function(self) -> Option<(EncodedType<&'a [u8]>, ParamsIter<'a>)> {
        let bytes = self.storage;
        if bytes.is_empty() {
            return None;
        }

        // Check wire tag
        let wire_tag = WireTag::from_byte(bytes[0]).ok()?;
        if wire_tag.type_tag != TypeTag::Function {
            return None;
        }

        // Extract payload with lifetime 'a from storage
        if bytes.len() < 3 {
            return None;
        }
        let size = read_u16_le(&bytes[1..3]) as usize;
        if bytes.len() < 3 + size {
            return None;
        }
        let payload: &'a [u8] = &bytes[3..3 + size];

        // Return type comes first
        let return_view = EncodedType::new(payload);
        let return_len = return_view.encoded_len();

        // Params iterator reads its own count and payload
        let params_iter = ParamsIter::new(&payload[return_len..]).ok()?;

        Some((return_view, params_iter))
    }

    pub fn as_symbol(self) -> Option<SymbolIter<'a>> {
        let bytes = self.storage;
        if bytes.is_empty() {
            return None;
        }

        // Check wire tag
        let wire_tag = WireTag::from_byte(bytes[0]).ok()?;
        if wire_tag.type_tag != TypeTag::Symbol {
            return None;
        }

        // Extract payload with lifetime 'a from storage
        if bytes.len() < 3 {
            return None;
        }
        let size = read_u16_le(&bytes[1..3]) as usize;
        if bytes.len() < 3 + size {
            return None;
        }
        let payload: &'a [u8] = &bytes[3..3 + size];
        SymbolIter::new(payload).ok()
    }
}

// ============================================================================
// Iterators
// ============================================================================

/// Validates that the iterator has consumed its entire payload when finished.
/// Returns an error if there are leftover bytes after all items are consumed.
fn validate_payload_is_empty(payload: &[u8]) -> Option<DecodeError> {
    if payload.is_empty() {
        return None;
    }
    Some(DecodeError::SizeMismatch {
        offset: 0,
        claimed: 0,            // Already consumed what we expected
        actual: payload.len(), // But there are leftover bytes
    })
}

pub struct RecordIter<'a> {
    payload: &'a [u8],
    remaining: usize,
}

impl<'a> RecordIter<'a> {
    pub fn new(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let (count, varint_len) = read_varint(payload)?;
        Ok(RecordIter {
            payload: &payload[varint_len..],
            remaining: count,
        })
    }
}

impl<'a> Iterator for RecordIter<'a> {
    type Item = Result<(&'a str, EncodedType<&'a [u8]>), DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            if let Some(e) = validate_payload_is_empty(self.payload) {
                return Some(Err(e));
            }
            return None;
        }
        self.remaining -= 1;

        let (name, name_len) = match read_string(self.payload) {
            Ok(r) => r,
            Err(e) => return Some(Err(e)),
        };

        let ty_view = EncodedType::new(&self.payload[name_len..]);
        let ty_len = ty_view.encoded_len();

        self.payload = &self.payload[name_len + ty_len..];

        Some(Ok((name, ty_view)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a> ExactSizeIterator for RecordIter<'a> {}

pub struct ParamsIter<'a> {
    payload: &'a [u8],
    remaining: usize,
}

impl<'a> ParamsIter<'a> {
    pub fn new(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let (count, varint_len) = read_varint(payload)?;
        Ok(ParamsIter {
            payload: &payload[varint_len..],
            remaining: count,
        })
    }
}

impl<'a> Iterator for ParamsIter<'a> {
    type Item = Result<EncodedType<&'a [u8]>, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            if let Some(e) = validate_payload_is_empty(self.payload) {
                return Some(Err(e));
            }
            return None;
        }

        self.remaining -= 1;

        if self.payload.is_empty() {
            return Some(Err(DecodeError::Truncated {
                offset: 0,
                needed: 1,
            }));
        }

        let view = EncodedType::new(self.payload);
        let len = view.encoded_len();
        self.payload = &self.payload[len..];

        Some(Ok(view))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a> ExactSizeIterator for ParamsIter<'a> {}

pub struct SymbolIter<'a> {
    payload: &'a [u8],
    remaining: usize,
}

impl<'a> SymbolIter<'a> {
    pub fn new(payload: &'a [u8]) -> Result<Self, DecodeError> {
        let (count, varint_len) = read_varint(payload)?;
        Ok(SymbolIter {
            payload: &payload[varint_len..],
            remaining: count,
        })
    }
}

impl<'a> Iterator for SymbolIter<'a> {
    type Item = Result<&'a str, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            if let Some(e) = validate_payload_is_empty(self.payload) {
                return Some(Err(e));
            }
            return None;
        }

        self.remaining -= 1;

        match read_string(self.payload) {
            Ok((part, part_len)) => {
                self.payload = &self.payload[part_len..];
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
// Decoding
// ============================================================================

pub fn decode<'a>(
    bytes: &'a [u8],
    mgr: &'a crate::types::manager::TypeManager<'a>,
) -> Result<&'a Type<'a>, DecodeError> {
    let view = EncodedType::new(bytes);
    let (ty, consumed) = decode_from_view(view, mgr, 0)?;

    // Check we consumed everything
    if consumed != bytes.len() {
        return Err(DecodeError::TrailingBytes {
            offset: consumed,
            remaining: bytes.len() - consumed,
        });
    }

    Ok(ty)
}

fn decode_from_view<'a>(
    view: EncodedType<&'a [u8]>,
    mgr: &'a crate::types::manager::TypeManager<'a>,
    depth: usize,
) -> Result<(&'a Type<'a>, usize), DecodeError> {
    const MAX_DEPTH: usize = 1000;

    if depth > MAX_DEPTH {
        return Err(DecodeError::TooDeep { depth });
    }

    let bytes = view.bytes();
    if bytes.is_empty() {
        return Err(DecodeError::Truncated {
            offset: 0,
            needed: 1,
        });
    }

    // Decode wire tag
    let wire_tag = WireTag::from_byte(bytes[0])?;

    match wire_tag.type_tag {
        TypeTag::Int => Ok((mgr.int(), 1)),
        TypeTag::Float => Ok((mgr.float(), 1)),
        TypeTag::Bool => Ok((mgr.bool(), 1)),
        TypeTag::Str => Ok((mgr.str(), 1)),
        TypeTag::Bytes => Ok((mgr.bytes(), 1)),

        TypeTag::TypeVar => {
            // Check for packed payload
            if let PayloadLocation::PackedTypeVar(id) = wire_tag.payload {
                Ok((mgr.type_var(id), 1))
            } else {
                // Long format - read from payload
                let id = view.as_typevar().ok_or(DecodeError::Truncated {
                    offset: 0,
                    needed: 1,
                })?;
                Ok((mgr.type_var(id), view.encoded_len()))
            }
        }

        TypeTag::Array => {
            // Future: handle packed arrays
            let elem_view = view.as_array().ok_or(DecodeError::Truncated {
                offset: 0,
                needed: 1,
            })?;
            let (elem, elem_consumed) = decode_from_view(elem_view, mgr, depth + 1)?;

            // For long format composite types, validate size field
            if matches!(wire_tag.payload, PayloadLocation::LongFormat) {
                validate_composite_size(view, elem_consumed)?;
            }

            Ok((mgr.array(elem), view.encoded_len()))
        }

        TypeTag::Map => {
            // Future: handle packed maps
            let (key_view, val_view) = view.as_map().ok_or(DecodeError::Truncated {
                offset: 0,
                needed: 1,
            })?;
            let (key, key_consumed) = decode_from_view(key_view, mgr, depth + 1)?;
            let (val, val_consumed) = decode_from_view(val_view, mgr, depth + 1)?;

            // For long format composite types, validate size field
            if matches!(wire_tag.payload, PayloadLocation::LongFormat) {
                let actual_size = key_consumed + val_consumed;
                validate_composite_size(view, actual_size)?;
            }

            Ok((mgr.map(key, val), view.encoded_len()))
        }

        TypeTag::Record => {
            let iter = view.as_record().ok_or(DecodeError::Truncated {
                offset: 0,
                needed: 1,
            })?;
            let mut fields = Vec::new();
            for result in iter {
                let (name, ty_view) = result?;
                let (ty, _) = decode_from_view(ty_view, mgr, depth + 1)?;
                fields.push((name, ty));
            }
            Ok((mgr.record(fields), view.encoded_len()))
        }

        TypeTag::Function => {
            let (ret_view, params_iter) = view.as_function().ok_or(DecodeError::Truncated {
                offset: 0,
                needed: 1,
            })?;

            // Decode return type first
            let (ret, _) = decode_from_view(ret_view, mgr, depth + 1)?;

            // Then decode params
            let mut params = Vec::new();
            for result in params_iter {
                let param_view = result?;
                let (param, _) = decode_from_view(param_view, mgr, depth + 1)?;
                params.push(param);
            }

            // Validate size field matches actual content
            validate_composite_size(view, view.payload().len())?;

            Ok((mgr.function(&params, ret), view.encoded_len()))
        }

        TypeTag::Symbol => {
            let iter = view.as_symbol().ok_or(DecodeError::Truncated {
                offset: 0,
                needed: 1,
            })?;
            let mut parts = Vec::new();

            for result in iter {
                let part = result?;
                parts.push(part);
            }

            // Validate size field matches actual content
            validate_composite_size(view, view.payload().len())?;

            Ok((mgr.symbol(parts), view.encoded_len()))
        }
    }
}

// ============================================================================
// TypeView Implementation
// ============================================================================

use crate::types::type_traits::{TypeKind, TypeView};

/// Wrapper around ParamsIter that panics on decode errors.
/// Used for TypeView implementation where we assume well-formed encoded data.
pub struct ParamsIterView<'a> {
    inner: ParamsIter<'a>,
}

impl<'a> ParamsIterView<'a> {
    fn new(inner: ParamsIter<'a>) -> Self {
        ParamsIterView { inner }
    }
}

impl<'a> Iterator for ParamsIterView<'a> {
    type Item = EncodedType<&'a [u8]>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|r| r.expect("invalid encoded type in params"))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> ExactSizeIterator for ParamsIterView<'a> {}

/// Wrapper around RecordIter that panics on decode errors.
pub struct RecordIterView<'a> {
    inner: RecordIter<'a>,
}

impl<'a> RecordIterView<'a> {
    fn new(inner: RecordIter<'a>) -> Self {
        RecordIterView { inner }
    }
}

impl<'a> Iterator for RecordIterView<'a> {
    type Item = (&'a str, EncodedType<&'a [u8]>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|r| r.expect("invalid encoded type in record"))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> ExactSizeIterator for RecordIterView<'a> {}

/// Wrapper around SymbolIter that panics on decode errors.
pub struct SymbolIterView<'a> {
    inner: SymbolIter<'a>,
}

impl<'a> SymbolIterView<'a> {
    fn new(inner: SymbolIter<'a>) -> Self {
        SymbolIterView { inner }
    }
}

impl<'a> Iterator for SymbolIterView<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|r| r.expect("invalid encoded symbol part"))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> ExactSizeIterator for SymbolIterView<'a> {}

impl<'a> TypeView<'a> for EncodedType<&'a [u8]> {
    type Iter = ParamsIterView<'a>;
    type NamedIter = RecordIterView<'a>;
    type StrIter = SymbolIterView<'a>;

    fn view(self) -> TypeKind<'a, Self> {
        let bytes = self.bytes();
        if bytes.is_empty() {
            // Fallback: treat as invalid Int (safer than panicking)
            return TypeKind::Int;
        }

        // Decode wire tag
        let wire_tag = match WireTag::from_byte(bytes[0]) {
            Ok(tag) => tag,
            Err(_) => return TypeKind::Int, // Invalid, fallback to Int
        };

        // Check for packed payload first
        if let PayloadLocation::PackedTypeVar(id) = wire_tag.payload {
            return TypeKind::TypeVar(id);
        }

        // Match on type tag
        match wire_tag.type_tag {
            TypeTag::Int => TypeKind::Int,
            TypeTag::Float => TypeKind::Float,
            TypeTag::Bool => TypeKind::Bool,
            TypeTag::Str => TypeKind::Str,
            TypeTag::Bytes => TypeKind::Bytes,

            TypeTag::TypeVar => {
                if let Some(id) = self.as_typevar() {
                    TypeKind::TypeVar(id)
                } else {
                    TypeKind::TypeVar(0) // Fallback for invalid encoding
                }
            }

            TypeTag::Array => {
                if let Some(elem) = self.as_array() {
                    TypeKind::Array(elem)
                } else {
                    TypeKind::Int // Fallback for invalid encoding
                }
            }

            TypeTag::Map => {
                if let Some((key, val)) = self.as_map() {
                    TypeKind::Map(key, val)
                } else {
                    TypeKind::Int // Fallback for invalid encoding
                }
            }

            TypeTag::Record => {
                if let Some(iter) = self.as_record() {
                    TypeKind::Record(RecordIterView::new(iter))
                } else {
                    TypeKind::Int // Fallback for invalid encoding
                }
            }

            TypeTag::Function => {
                if let Some((ret, params)) = self.as_function() {
                    TypeKind::Function {
                        params: ParamsIterView::new(params),
                        ret,
                    }
                } else {
                    TypeKind::Int // Fallback for invalid encoding
                }
            }

            TypeTag::Symbol => {
                if let Some(iter) = self.as_symbol() {
                    TypeKind::Symbol(SymbolIterView::new(iter))
                } else {
                    TypeKind::Int // Fallback for invalid encoding
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::manager::TypeManager;
    use bumpalo::Bump;

    #[test]
    fn test_smallvec_size() {
        dbg!(core::mem::size_of::<SmallVec<[u8; 1]>>());
        dbg!(core::mem::size_of::<SmallVec<[u8; 8]>>());
        dbg!(core::mem::size_of::<SmallVec<[u8; 16]>>());
        dbg!(core::mem::size_of::<SmallVec<[u8; 18]>>());
        dbg!(core::mem::size_of::<SmallVec<[u8; 24]>>());
        assert!(true);
    }

    // ============================================================================
    // Packed Type Navigation Tests (CRITICAL)
    // ============================================================================

    #[test]
    fn test_discriminant_normalization() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Array[Int] uses packed format (1 byte)
        let ty = mgr.array(mgr.int());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed: single byte

        // Verify wire byte decodes to correct type
        let wire_tag = WireTag::from_byte(bytes[0]).unwrap();
        assert_eq!(wire_tag.type_tag, TypeTag::Array);

        let view = EncodedType::new(&bytes[..]);
        assert!(matches!(view.view(), TypeKind::Array(_)));

        // Map[Str, Int] uses packed format (1 byte)
        let ty = mgr.map(mgr.str(), mgr.int());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed: single byte

        let wire_tag = WireTag::from_byte(bytes[0]).unwrap();
        assert_eq!(wire_tag.type_tag, TypeTag::Map);

        let view = EncodedType::new(&bytes[..]);
        assert!(matches!(view.view(), TypeKind::Map(_, _)));
    }

    #[test]
    fn test_navigate_packed_array_int() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Array[Int] - packed encoding (1 byte)
        let ty = mgr.array(mgr.int());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed: single byte

        let view = EncodedType::new(&bytes[..]);

        // This should work - uniform access!
        let elem_view = view.as_array().expect("should be an array");
        assert!(matches!(elem_view.view(), TypeKind::Int));
    }

    #[test]
    fn test_navigate_packed_map_str_int() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Map[Str, Int] - packed encoding (1 byte)
        let ty = mgr.map(mgr.str(), mgr.int());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed: single byte

        let view = EncodedType::new(&bytes[..]);

        // This should work - uniform access!
        let (key_view, val_view) = view.as_map().expect("should be a map");
        assert!(matches!(key_view.view(), TypeKind::Str));
        assert!(matches!(val_view.view(), TypeKind::Int));
    }

    #[test]
    fn test_navigate_all_packed_arrays() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Test Int
        let ty = mgr.array(mgr.int());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let view = EncodedType::new(&bytes[..]);
        assert!(matches!(view.view(), TypeKind::Array(_)));
        let elem_view = view.as_array().expect("should be an array");
        assert!(matches!(elem_view.view(), TypeKind::Int));

        // Test Float
        let ty = mgr.array(mgr.float());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let view = EncodedType::new(&bytes[..]);
        let elem_view = view.as_array().expect("should be an array");
        assert!(matches!(elem_view.view(), TypeKind::Float));

        // Test Bool
        let ty = mgr.array(mgr.bool());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let view = EncodedType::new(&bytes[..]);
        let elem_view = view.as_array().expect("should be an array");
        assert!(matches!(elem_view.view(), TypeKind::Bool));

        // Test Str
        let ty = mgr.array(mgr.str());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let view = EncodedType::new(&bytes[..]);
        let elem_view = view.as_array().expect("should be an array");
        assert!(matches!(elem_view.view(), TypeKind::Str));

        // Test Bytes
        let ty = mgr.array(mgr.bytes());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let view = EncodedType::new(&bytes[..]);
        let elem_view = view.as_array().expect("should be an array");
        assert!(matches!(elem_view.view(), TypeKind::Bytes));
    }

    #[test]
    fn test_navigate_all_packed_maps() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Test a few representative map combinations
        // Map[Int, Int]
        let ty = mgr.map(mgr.int(), mgr.int());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let view = EncodedType::new(&bytes[..]);
        assert!(matches!(view.view(), TypeKind::Map(_, _)));
        let (key_view, val_view) = view.as_map().expect("should be a map");
        assert!(matches!(key_view.view(), TypeKind::Int));
        assert!(matches!(val_view.view(), TypeKind::Int));

        // Map[Str, Float]
        let ty = mgr.map(mgr.str(), mgr.float());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let view = EncodedType::new(&bytes[..]);
        let (key_view, val_view) = view.as_map().expect("should be a map");
        assert!(matches!(key_view.view(), TypeKind::Str));
        assert!(matches!(val_view.view(), TypeKind::Float));

        // Map[Bool, Bytes]
        let ty = mgr.map(mgr.bool(), mgr.bytes());
        let bytes = encode(ty);
        assert_eq!(bytes.len(), 1); // Packed
        let view = EncodedType::new(&bytes[..]);
        let (key_view, val_view) = view.as_map().expect("should be a map");
        assert!(matches!(key_view.view(), TypeKind::Bool));
        assert!(matches!(val_view.view(), TypeKind::Bytes));
    }

    // ============================================================================
    // TypeView-based Decode Tests
    // ============================================================================

    #[test]
    fn test_decode_via_typeview_primitives() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let int_bytes = encode(mgr.int());
        let decoded = decode(&int_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.int(), decoded));

        let float_bytes = encode(mgr.float());
        let decoded = decode(&float_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.float(), decoded));

        let bool_bytes = encode(mgr.bool());
        let decoded = decode(&bool_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.bool(), decoded));

        let str_bytes = encode(mgr.str());
        let decoded = decode(&str_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.str(), decoded));

        let bytes_bytes = encode(mgr.bytes());
        let decoded = decode(&bytes_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.bytes(), decoded));
    }

    #[test]
    fn test_decode_via_typeview_packed_array() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.array(mgr.int());
        let bytes = encode(ty);

        // decode() uses only TypeView navigation
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_decode_via_typeview_packed_map() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.map(mgr.str(), mgr.int());
        let bytes = encode(ty);

        // decode() uses only TypeView navigation
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_decode_via_typeview_complex() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

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
    // Lenient Decoding Tests
    // ============================================================================

    #[test]
    fn test_lenient_decode_non_packed_array_int() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Manually create encoding: [Array wire byte][size][Int wire byte]
        let array_byte = WireTag::to_wire_byte(TypeTag::Array);
        let int_byte = WireTag::to_wire_byte(TypeTag::Int);
        let bytes = vec![array_byte, 1, 0, int_byte];

        let decoded = decode(&bytes, &mgr).unwrap();
        let expected = mgr.array(mgr.int());

        // Should intern to same pointer as canonical encoding
        assert!(core::ptr::eq(decoded, expected));
    }

    #[test]
    fn test_lenient_decode_non_packed_map() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Manually create encoding: [Map wire byte][size][Str wire byte][Int wire byte]
        let map_byte = WireTag::to_wire_byte(TypeTag::Map);
        let str_byte = WireTag::to_wire_byte(TypeTag::Str);
        let int_byte = WireTag::to_wire_byte(TypeTag::Int);
        let bytes = vec![map_byte, 2, 0, str_byte, int_byte];

        let decoded = decode(&bytes, &mgr).unwrap();
        let expected = mgr.map(mgr.str(), mgr.int());

        // Should intern to same pointer as canonical encoding
        assert!(core::ptr::eq(decoded, expected));
    }

    #[test]
    fn test_both_encodings_intern_same() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Canonical encoding now uses packed format (1 byte)
        let ty = mgr.array(mgr.int());
        let canonical_bytes = encode(ty);
        assert_eq!(canonical_bytes.len(), 1); // Packed format

        // Alternative long-format encoding should decode to same interned type
        let array_byte = WireTag::to_wire_byte(TypeTag::Array);
        let int_byte = WireTag::to_wire_byte(TypeTag::Int);
        let alternative_bytes = vec![array_byte, 1, 0, int_byte]; // Long format: [disc][size_lo][size_hi][elem]

        let decoded_canonical = decode(&canonical_bytes, &mgr).unwrap();
        let decoded_alternative = decode(&alternative_bytes, &mgr).unwrap();

        // Both should intern to same pointer (lenient decoding)
        assert!(core::ptr::eq(ty, decoded_canonical));
        assert!(core::ptr::eq(ty, decoded_alternative));
        assert!(core::ptr::eq(decoded_canonical, decoded_alternative));
    }

    // ============================================================================
    // Basic Round-trip Tests
    // ============================================================================

    #[test]
    fn test_primitives_round_trip() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let int_bytes = encode(mgr.int());
        let decoded = decode(&int_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.int(), decoded));

        let float_bytes = encode(mgr.float());
        let decoded = decode(&float_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.float(), decoded));

        let bool_bytes = encode(mgr.bool());
        let decoded = decode(&bool_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.bool(), decoded));

        let str_bytes = encode(mgr.str());
        let decoded = decode(&str_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.str(), decoded));

        let bytes_bytes = encode(mgr.bytes());
        let decoded = decode(&bytes_bytes, &mgr).unwrap();
        assert!(core::ptr::eq(mgr.bytes(), decoded));
    }

    #[test]
    fn test_typevar_round_trip() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Test packed and non-packed
        let ty0 = mgr.type_var(0);
        let bytes0 = encode(ty0);
        let decoded0 = decode(&bytes0, &mgr).unwrap();
        assert!(core::ptr::eq(ty0, decoded0));

        let ty15 = mgr.type_var(15);
        let bytes15 = encode(ty15);
        let decoded15 = decode(&bytes15, &mgr).unwrap();
        assert!(core::ptr::eq(ty15, decoded15));

        let ty31 = mgr.type_var(31);
        let bytes31 = encode(ty31);
        let decoded31 = decode(&bytes31, &mgr).unwrap();
        assert!(core::ptr::eq(ty31, decoded31));

        let ty32 = mgr.type_var(32);
        let bytes32 = encode(ty32);
        let decoded32 = decode(&bytes32, &mgr).unwrap();
        assert!(core::ptr::eq(ty32, decoded32));

        let ty100 = mgr.type_var(100);
        let bytes100 = encode(ty100);
        let decoded100 = decode(&bytes100, &mgr).unwrap();
        assert!(core::ptr::eq(ty100, decoded100));

        let ty1000 = mgr.type_var(1000);
        let bytes1000 = encode(ty1000);
        let decoded1000 = decode(&bytes1000, &mgr).unwrap();
        assert!(core::ptr::eq(ty1000, decoded1000));
    }

    #[test]
    fn test_record_round_trip() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.record(vec![("age", mgr.int()), ("name", mgr.str())]);

        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_function_round_trip() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.function(&[mgr.int(), mgr.str()], mgr.bool());

        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_symbol_round_trip() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.symbol(vec!["error", "pending", "success"]);
        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    // ============================================================================
    // TypeView Navigation Tests (Non-Packed)
    // ============================================================================

    #[test]
    fn test_navigate_non_packed_array() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.array(mgr.array(mgr.int()));
        let bytes = encode(ty);

        let view = EncodedType::new(&bytes[..]);
        let elem_view = view.as_array().unwrap();

        // Nested array
        let inner_elem = elem_view.as_array().unwrap();
        assert!(matches!(inner_elem.view(), TypeKind::Int));
    }

    #[test]
    fn test_navigate_non_packed_map() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.map(mgr.array(mgr.int()), mgr.str());
        let bytes = encode(ty);

        let view = EncodedType::new(&bytes[..]);
        let (key, val) = view.as_map().unwrap();

        assert!(matches!(key.view(), TypeKind::Array(_)));
        assert!(matches!(val.view(), TypeKind::Str));
    }

    #[test]
    fn test_navigate_record() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.record(vec![("age", mgr.int()), ("name", mgr.str())]);

        let bytes = encode(ty);
        let view = EncodedType::new(&bytes[..]);

        let fields: Vec<_> = view.as_record().unwrap().map(|r| r.unwrap()).collect();

        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].0, "age");
        assert!(matches!(fields[0].1.view(), TypeKind::Int));
        assert_eq!(fields[1].0, "name");
        assert!(matches!(fields[1].1.view(), TypeKind::Str));
    }

    #[test]
    fn test_navigate_function() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.function(&[mgr.int(), mgr.str()], mgr.bool());

        let bytes = encode(ty);
        let view = EncodedType::new(&bytes[..]);

        let (ret, params_iter) = view.as_function().unwrap();

        let params: Vec<_> = params_iter.map(|r| r.unwrap()).collect();
        assert_eq!(params.len(), 2);
        assert!(matches!(params[0].view(), TypeKind::Int));
        assert!(matches!(params[1].view(), TypeKind::Str));

        assert!(matches!(ret.view(), TypeKind::Bool));
    }

    #[test]
    fn test_navigate_symbol() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        // Note: TypeManager sorts symbol parts
        let ty = mgr.symbol(vec!["success", "error", "pending"]);
        let bytes = encode(ty);
        let view = EncodedType::new(&bytes[..]);

        let parts: Vec<_> = view.as_symbol().unwrap().map(|r| r.unwrap()).collect();

        assert_eq!(parts.len(), 3);
        // Sorted order
        assert_eq!(parts[0], "error");
        assert_eq!(parts[1], "pending");
        assert_eq!(parts[2], "success");
    }

    // ============================================================================
    // Error Handling Tests
    // ============================================================================

    #[test]
    fn test_decode_empty_buffer() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let result = decode(&[], &mgr);
        assert!(matches!(result, Err(DecodeError::Truncated { .. })));
    }

    #[test]
    fn test_decode_trailing_bytes() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.int();
        let mut bytes = encode(ty).to_vec();
        bytes.push(0xFF); // Extra byte

        let result = decode(&bytes, &mgr);
        assert!(matches!(result, Err(DecodeError::TrailingBytes { .. })));
    }

    #[test]
    fn test_decode_unknown_discriminant() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let bytes = vec![37]; // Reserved slot
        let result = decode(&bytes, &mgr);
        assert!(matches!(
            result,
            Err(DecodeError::UnknownDiscriminant { .. })
        ));
    }

    #[test]
    fn test_decode_truncated_composite() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let array_byte = WireTag::to_wire_byte(TypeTag::Array);
        let bytes = vec![array_byte, 5, 0]; // Claims 5 bytes but no payload
        let result = decode(&bytes, &mgr);
        assert!(matches!(result, Err(DecodeError::Truncated { .. })));
    }

    // ============================================================================
    // Edge Cases
    // ============================================================================

    #[test]
    fn test_empty_record() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.record(vec![]);
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
    fn test_deeply_nested() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let mut ty = mgr.int();
        for _ in 0..100 {
            ty = mgr.array(ty);
        }

        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_unicode_field_names() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.record(vec![
            ("name", mgr.str()),
            ("名前", mgr.str()),
            ("🎉", mgr.bool()),
        ]);

        let bytes = encode(ty);
        let decoded = decode(&bytes, &mgr).unwrap();
        assert!(core::ptr::eq(ty, decoded));
    }

    #[test]
    fn test_encode_deterministic() {
        let arena = Bump::new();
        let mgr = TypeManager::new(&arena);

        let ty = mgr.function(&[mgr.map(mgr.str(), mgr.int())], mgr.array(mgr.bool()));

        let bytes1 = encode(ty);
        let bytes2 = encode(ty);
        let bytes3 = encode(ty);

        assert_eq!(bytes1.as_slice(), bytes2.as_slice());
        assert_eq!(bytes2.as_slice(), bytes3.as_slice());
    }
}
