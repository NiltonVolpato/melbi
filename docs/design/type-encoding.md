---
title: Melbi Type Encoding Format - Technical Design Document
---

# Design Doc: Melbi Type Encoding Format

**Author**: Claude (with Nilton)

**Date**: 11-02-2025

## Introduction

### Background

Melbi types are currently represented as `&'a Type<'a>` with arena allocation and interning. While this provides excellent performance for type checking and inference (O(1) pointer equality), it has a critical limitation: the lifetime `'a` propagates throughout the entire codebase. This makes it difficult to:

1. **Cross boundaries**: Store types in structures that outlive the arena, send types across thread boundaries, or serialize types for caching
2. **Compare across contexts**: Compare types from different `TypeManager` instances without complex adoption logic
3. **Navigate efficiently**: Pattern match on type structure without full deserialization

We need an alternate representation that:
- Has **no lifetime dependencies** (just `&[u8]` or `Vec<u8>`)
- Supports **fast equality** via byte comparison
- Enables **zero-copy navigation** for pattern matching
- Remains **compact** for memory efficiency
- **Roundtrips perfectly** with the arena representation

### Current Functionality

Types are represented as:
```rust
enum Type<'a> {
    Int, Float, Bool, Str, Bytes,
    Array(&'a Type<'a>),
    Map(&'a Type<'a>, &'a Type<'a>),
    Record(&'a [(&'a str, &'a Type<'a>)]),
    Function { params: &'a [&'a Type<'a>], ret: &'a Type<'a> },
    Symbol(&'a [&'a str]),
    TypeVar(u16),
}
```

The `TypeManager` interns all types, so:
- Type equality = pointer equality (O(1))
- Types share the arena lifetime `'a`
- Pattern matching is natural via Rust's `match`

### In Scope

This design addresses:
1. **Binary encoding format** for Melbi types
2. **TypeView API** for zero-copy navigation and pattern matching
3. **Encode/decode functions** for converting between representations
4. **Error handling** for malformed encodings
5. **Comprehensive test suite** including fuzz testing

### Out of Scope

- **Versioning**: This is v1 of the format; future versions will use bit 7
- **Compression**: Types are already compact; compression is a separate concern
- **Streaming encode/decode**: We encode/decode complete types
- **Cross-language compatibility**: This is a Melbi-internal format

### Assumptions & Dependencies

- **Arena allocation** remains the primary representation for active type checking
- **Interning** continues to provide O(1) equality for arena types
- **bumpalo** crate provides arena allocation
- **smallvec** crate provides stack-allocated vectors (most types fit in 16 bytes)
- Types are **structurally finite** (no infinite recursion)

### Terminology

- **Discriminant**: Single byte indicating type variant
- **Packed encoding**: Type encoded in discriminant byte alone (1 byte total)
- **Composite encoding**: Type with size prefix and payload
- **TypeView**: Zero-copy view over encoded bytes
- **Interning**: Deduplication ensuring structural equality = pointer equality
- **Unitary type**: Type with no children (primitives, packed arrays/maps, typevars)
- **Composite type**: Type with children requiring recursive encoding

## Considerations

### Concerns

**Performance**:
- Encoding must be fast enough for hot paths
- Decoding should be lazy (TypeView) with full decode only when needed
- Byte equality must be reliable (canonical encoding required)

**Correctness**:
- Must handle all type variants correctly
- Size fields must match actual content
- No buffer overruns on malformed input
- Interning must be preserved through round-trip

**Usability**:
- TypeView API must be ergonomic
- Error messages must be clear
- Format must be debuggable (human can read hex dump)

**Extensibility**:
- Must reserve space for future type variants
- Bit 7 reserved for format versioning
- Should minimize breaking changes

### Operational Readiness Considerations

**Deployment**: This is a library feature, deployed via Cargo.

**Monitoring**: Not applicable - library code.

**Error handling**: All decode operations return `Result<T, DecodeError>` with detailed error information including byte offsets.

**Data recovery**: Encoding is deterministic and stateless. No recovery needed.

**Debugging**:
- Hex dump utilities for encoded types
- Pretty-printer showing structure
- Validation functions checking invariants

**Multi-tenancy**: Not applicable.

**Throttling**: Not applicable.

### Open Questions

1. **Should we support non-canonical encodings during decode?**
   - Answer: Yes, be lenient. Accept both `[Array][Int]` and packed `[ArrayInt]`

2. **How do we handle very large types (>64KB)?**
   - Answer: u16 size field limits individual nodes to 64KB. Deeply nested types compose fine.

3. **Should TypeView methods return iterators or collections?**
   - Answer: Iterators. More flexible and zero-copy.

4. **Do we need a schema/version byte?**
   - Answer: Not in v1. Bit 7 reserved for future versioning.

### Cross-Region Considerations

Not applicable - this is an in-memory format.

## Proposed Design

### Solution

We define a **compact binary encoding** with two key components:

1. **Encoding format**: Byte representation with discriminant, size prefixes, and payload
2. **TypeView API**: Zero-copy iterator-based navigation over encoded bytes

**Key properties**:
- Discriminant layout uses power-of-2 boundaries for fast category detection
- Common types (primitives, simple arrays/maps) packed into single byte
- Size prefixes enable skipping subtrees without parsing
- Canonical encoding ensures byte equality = structural equality
- No overlapping slices - each type owns its byte range exclusively

### System Architecture

```
┌─────────────────┐
│  Type<'a>       │
│  (Arena-based)  │
└────────┬────────┘
         │
    encode│    ┌──────────────┐
         ├───>│  SmallVec     │
         │    │  [u8; 16]     │
    ┌────┴───>│  (bytes)      │
    │         └───────┬────────┘
decode│                │
    │         ┌────────▼───────┐
    │         │  TypeView<'a>  │
    │         │  (zero-copy)   │
    │         └────────┬───────┘
    │                  │
    │         ┌────────▼───────────┐
    │         │  Pattern matching  │
    │         │  via iterators     │
    └─────────┴────────────────────┘
```

### Data Model

#### Discriminant Layout (0-127, bit 7 reserved)

```
0-31   (0b000xxxxx): TypeVar(0..31) - packed type variables
32-63  (0b001xxxxx): Unitary types - primitives with no children
  32: Int
  33: Float
  34: Bool
  35: Str
  36: Bytes
  37-63: Reserved for future unitary types

64-95  (0b010xxxxx): Composite types - types with children
  64: Array
  65: Map
  66: Record
  67: Function
  68: Symbol
  69-95: Reserved for future composite types

96-127 (0b011xxxxx): Packed types - common patterns
  96-100:   Array[Primitive] (5 variants)
    96:  Array[Int]
    97:  Array[Float]
    98:  Array[Bool]
    99:  Array[Str]
    100: Array[Bytes]
  101-125: Map[Primitive, Primitive] (25 variants)
    Formula: 101 + (key_idx * 5) + val_idx
    Where: Int=0, Float=1, Bool=2, Str=3, Bytes=4
  126-127: Reserved
```

**Category detection** via bit operations:
```rust
let category = disc >> 5;
// 0 = TypeVar
// 1 = Unitary
// 2 = Composite
// 3 = Packed
```

#### Encoding Formats

**Unitary types** (no children):
```
[discriminant: u8]
```
Total: 1 byte

Examples:
- `Int` → `[32]`
- `Float` → `[33]`
- `TypeVar(5)` → `[5]`
- `Array[Int]` → `[96]` (packed)
- `Map[Str, Bool]` → `[118]` (packed: 101 + 3*5 + 2)

**Composite types** (with children):
```
[discriminant: u8][size: u16 little-endian][payload: ...]
```
Where:
- `size` = length of payload (excludes discriminant and size bytes)
- `payload` = concatenated encodings of children plus metadata

**Array**:
```
[64][size: u16][elem_encoding: ...]
```

**Map**:
```
[65][size: u16][key_encoding: ...][val_encoding: ...]
```

**Record**:
```
[66][size: u16][field_count: varint]([field_name_len: varint][field_name: utf8][field_type: ...])*
```

**Function**:
```
[67][size: u16][param_count: varint]([param_type: ...])*[ret_type: ...]
```

**Symbol**:
```
[68][size: u16][part_count: varint]([part_len: varint][part: utf8])*
```

**TypeVar (non-packed)**:
```
[69][size: u16][id: u16 little-endian]
```
Note: TypeVar discriminant moves from 10 → 69 in new design

**Varint encoding**: Standard LEB128 (7 bits per byte, high bit = continuation)

#### Properties

1. **No overlapping slices**: Each type's encoding is a contiguous byte range
2. **Canonical**: Same type always encodes to same bytes
3. **Self-describing**: Can decode without external schema
4. **Bounded**: Maximum single-node size is 64KB (u16)
5. **Composable**: Deeply nested types work fine

### Interface / API Definitions

#### Encoding API

```rust
/// Encode a type to bytes. Most types fit in 16 bytes (no heap allocation).
pub fn encode(ty: &Type) -> SmallVec<[u8; 16]>;
```

**Guarantees**:
- Deterministic (same type → same bytes)
- Never panics on valid types
- Returns canonical encoding

#### TypeView API

```rust
/// Zero-copy view over encoded type bytes
pub struct TypeView<'a> {
    bytes: &'a [u8],
}

impl<'a> TypeView<'a> {
    /// Create view from bytes. Does not validate.
    pub fn new(bytes: &'a [u8]) -> Self;

    /// Validate encoding and get view. Checks:
    /// - Size fields match actual content
    /// - No buffer overruns
    /// - Valid discriminants
    /// - Proper varint encoding
    pub fn validated(bytes: &'a [u8]) -> Result<Self, DecodeError>;

    /// Get discriminant byte
    pub fn discriminant(&self) -> u8;

    /// Get category (TypeVar=0, Unitary=1, Composite=2, Packed=3)
    pub fn category(&self) -> u8;

    /// Check if type is unitary (no children to navigate)
    pub fn is_unitary(&self) -> bool;

    /// For composite types: get size of payload
    pub fn size(&self) -> Option<u16>;

    /// Total encoded length in bytes
    pub fn encoded_len(&self) -> usize;

    // Type-specific accessors

    /// If this is an array, return element type view
    /// Works for both packed and non-packed arrays
    pub fn as_array(&self) -> Option<TypeView<'a>>;

    /// If this is a map, return (key, value) views
    /// Works for both packed and non-packed maps
    pub fn as_map(&self) -> Option<(TypeView<'a>, TypeView<'a>)>;

    /// If this is a record, return iterator over fields
    pub fn as_record(&self) -> Option<RecordIter<'a>>;

    /// If this is a function, return iterator over params + return type
    pub fn as_function(&self) -> Option<FunctionView<'a>>;

    /// If this is a symbol, return iterator over parts
    pub fn as_symbol(&self) -> Option<SymbolIter<'a>>;

    /// If this is a type variable, return ID
    pub fn as_typevar(&self) -> Option<u16>;
}

/// Iterator over record fields
pub struct RecordIter<'a> {
    payload: &'a [u8],
    remaining: usize,
    pos: usize,
}

impl<'a> Iterator for RecordIter<'a> {
    type Item = Result<(&'a str, TypeView<'a>), DecodeError>;
    fn next(&mut self) -> Option<Self::Item>;
}

/// Function view with params iterator + return type
pub struct FunctionView<'a> {
    payload: &'a [u8],
    param_count: usize,
    pos: usize,
}

impl<'a> FunctionView<'a> {
    /// Iterator over parameter types
    pub fn params(&self) -> ParamsIter<'a>;

    /// Return type (computed by skipping all params)
    pub fn return_type(&self) -> Result<TypeView<'a>, DecodeError>;
}

pub struct ParamsIter<'a> { /* ... */ }

impl<'a> Iterator for ParamsIter<'a> {
    type Item = Result<TypeView<'a>, DecodeError>;
    fn next(&mut self) -> Option<Self::Item>;
}

/// Iterator over symbol parts
pub struct SymbolIter<'a> {
    payload: &'a [u8],
    remaining: usize,
    pos: usize,
}

impl<'a> Iterator for SymbolIter<'a> {
    type Item = Result<&'a str, DecodeError>;
    fn next(&mut self) -> Option<Self::Item>;
}
```

#### Decoding API

```rust
/// Decode bytes into TypeManager, with full validation
pub fn decode<'a>(
    bytes: &[u8],
    mgr: &'a TypeManager<'a>
) -> Result<&'a Type<'a>, DecodeError>;

/// Error type for decode failures
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Hit end of buffer unexpectedly
    Truncated { offset: usize, needed: usize },

    /// Size field doesn't match actual content
    SizeMismatch { offset: usize, claimed: usize, actual: usize },

    /// Invalid discriminant byte
    UnknownDiscriminant { discriminant: u8, offset: usize },

    /// Invalid UTF-8 in string
    InvalidUtf8 { offset: usize },

    /// Invalid varint encoding
    InvalidVarint { offset: usize },

    /// Exceeded maximum recursion depth
    TooDeep { depth: usize },
}

impl std::fmt::Display for DecodeError { /* ... */ }
impl std::error::Error for DecodeError {}
```

### Business Logic

#### Encoding Algorithm

```rust
fn encode_inner(ty: &Type, buf: &mut SmallVec<[u8; 16]>) {
    match ty {
        // Try packed encodings first
        Type::TypeVar(id) if *id < 32 => {
            buf.push(*id as u8);
        }
        Type::Int => buf.push(32),
        Type::Float => buf.push(33),
        // ... other primitives ...

        Type::Array(elem) => {
            match **elem {
                Type::Int => buf.push(96),
                // ... other packed arrays ...
                _ => {
                    // Non-packed array
                    let start = buf.len();
                    buf.push(64);  // Array discriminant
                    buf.push(0);   // size placeholder (low)
                    buf.push(0);   // size placeholder (high)
                    encode_inner(elem, buf);
                    // Backpatch size
                    let size = buf.len() - start - 3;
                    buf[start + 1] = (size & 0xFF) as u8;
                    buf[start + 2] = ((size >> 8) & 0xFF) as u8;
                }
            }
        }

        // ... similar for other types ...
    }
}
```

#### Decoding Algorithm

```rust
fn decode_inner(bytes: &[u8], mgr: &TypeManager) -> Result<(&Type, usize), DecodeError> {
    if bytes.is_empty() {
        return Err(DecodeError::Truncated { offset: 0, needed: 1 });
    }

    let disc = bytes[0] & 0x7F;
    let category = disc >> 5;

    match category {
        0 => {
            // TypeVar
            let id = disc as u16;
            Ok((mgr.type_var(id), 1))
        }
        1 => {
            // Unitary
            match disc {
                32 => Ok((mgr.int(), 1)),
                33 => Ok((mgr.float(), 1)),
                // ...
                _ => Err(DecodeError::UnknownDiscriminant { discriminant: disc, offset: 0 })
            }
        }
        2 => {
            // Composite
            if bytes.len() < 3 {
                return Err(DecodeError::Truncated { offset: 0, needed: 3 });
            }
            let size = read_u16_le(&bytes[1..3]);
            if bytes.len() < 3 + size as usize {
                return Err(DecodeError::Truncated {
                    offset: 0,
                    needed: 3 + size as usize
                });
            }
            let payload = &bytes[3..3 + size as usize];

            match disc {
                64 => {
                    // Array
                    let (elem, elem_len) = decode_inner(payload, mgr)?;
                    if elem_len != size as usize {
                        return Err(DecodeError::SizeMismatch {
                            offset: 0,
                            claimed: size as usize,
                            actual: elem_len,
                        });
                    }
                    Ok((mgr.array(elem), 3 + size as usize))
                }
                // ... other composite types with size validation ...
            }
        }
        3 => {
            // Packed
            if (96..=100).contains(&disc) {
                // Packed array
                let prim_idx = disc - 96;
                let elem = get_primitive(prim_idx, mgr);
                Ok((mgr.array(elem), 1))
            } else if (101..=125).contains(&disc) {
                // Packed map
                let idx = disc - 101;
                let key_idx = idx / 5;
                let val_idx = idx % 5;
                let key = get_primitive(key_idx, mgr);
                let val = get_primitive(val_idx, mgr);
                Ok((mgr.map(key, val), 1))
            } else {
                Err(DecodeError::UnknownDiscriminant { discriminant: disc, offset: 0 })
            }
        }
        _ => unreachable!("category is 0-3"),
    }
}
```

### Migration Strategy

This is a new feature, not a migration. Existing code using `&'a Type<'a>` continues to work unchanged.

### Work Required

1. **Core encoding module** (~300 LOC)
   - Discriminant constants and helpers
   - `encode()` function with backpatching
   - Varint read/write utilities

2. **TypeView implementation** (~400 LOC)
   - Core TypeView struct
   - Category detection
   - Iterator implementations for each type

3. **Decoding with validation** (~300 LOC)
   - `decode()` function
   - `DecodeError` type
   - Size validation
   - Bounds checking

4. **Tests** (~500 LOC)
   - Round-trip tests for all type variants
   - Malformed input tests
   - Fuzz testing
   - Property-based tests

5. **Documentation** (~200 LOC)
   - API docs with examples
   - Format specification
   - Migration guide

**Total estimate**: ~1700 LOC

**Dependencies**: None (uses existing melbi-core infrastructure)

### Work Sequence

1. ✅ Design document (this document)
2. Implement discriminant constants and category helpers
3. Implement `encode()` with tests
4. Implement `TypeView` navigation with tests
5. Implement `decode()` with validation and tests
6. Add iterator implementations
7. Fuzz testing
8. Documentation
9. Integration with existing codebase

### High-level Test Plan

**Unit tests**:
- Encode each type variant, verify bytes
- Decode each type variant, verify pointer equality
- TypeView navigation for all types
- Iterator consumption

**Round-trip tests**:
- For every type: encode → decode → verify `ptr::eq(original, decoded)`
- Verify interning is preserved

**Error handling tests**:
- Truncated buffers at every possible offset
- Invalid discriminants
- Size mismatches
- Invalid UTF-8
- Invalid varints

**Property-based tests** (using `proptest`):
- ∀ valid type: `decode(encode(t)) == t` (pointer equality)
- ∀ valid type: `encode(t1) == encode(t2)` ⟺ `t1 == t2` (structural equality)
- ∀ valid type: encoded length matches size field

**Fuzz testing**:
- Random byte sequences fed to decoder
- Must either decode successfully or return specific error
- No panics or UB

### Deployment Sequence

1. Merge encoding module (no breaking changes)
2. Add to public API
3. Update documentation
4. Release as minor version bump

## Impact

### Performance

**Encoding**:
- Primitives: ~1ns (single byte write)
- Complex types: ~100ns (allocation + recursive encoding)
- Most types fit in 16 bytes (no heap allocation with SmallVec)

**Byte equality**:
- Simple memcmp, exits on first difference
- Typical: ~2-10ns depending on type complexity
- Still 3-5x slower than pointer equality but acceptable

**TypeView navigation**:
- Zero allocation
- O(1) for discriminant/category check
- O(1) for Map key/value access
- O(n) for Record/Function iteration (unavoidable)

**Decoding**:
- With validation: ~200ns for complex types
- Without validation (unsafe): ~100ns
- Interning means decoded types reuse existing instances

### Cost Analysis

Not applicable - library code, no runtime costs.

### Security

**Malformed input**:
- All decoding is bounds-checked
- No buffer overruns possible
- Returns specific errors (never panics)
- No undefined behavior on bad input

**DoS resistance**:
- Size fields limit individual nodes (u16 max)
- Recursion depth limited (configurable)
- No exponential blow-up possible

## Alternatives

### Alternative 1: Use existing serialization (postcard/bincode)

**Pros**: Less code to maintain, battle-tested
**Cons**:
- No zero-copy navigation
- Larger encoding (not optimized for our use case)
- Harder to implement packed discriminants

**Decision**: Rejected - we need custom format for TypeView

### Alternative 2: Indices instead of references

Use `Vec<Type>` with indices instead of arena references.

**Pros**: No lifetimes
**Cons**:
- Lose type safety (indices can be invalid)
- Lose O(1) equality (must compare structurally)
- More complex API

**Decision**: Rejected - indices are worse than serialization

### Alternative 3: Postcard for serialization + custom TypeView

Use postcard but add TypeView wrapper.

**Pros**: Less encoding code
**Cons**:
- Postcard format not optimized for our layout
- Can't implement packed discriminants
- Size overhead (~20% larger)

**Decision**: Rejected - size matters for hot paths

## Looking into the Future

### Future enhancements

1. **Compression**: Add optional zstd compression for disk storage
2. **Versioning**: Use bit 7 for format version when we need breaking changes
3. **More packed types**: Add packed Function types, common Record schemas
4. **Streaming**: Support encoding/decoding from `Read`/`Write` traits
5. **mmap support**: Design format to be directly usable from mmap'd files

### Next iterations

1. **v1.0**: This design (MVP)
2. **v1.1**: Add compression support
3. **v1.2**: Add common Record schema packing
4. **v2.0**: Add versioning and breaking changes if needed

### Nice to have

- Pretty-printer for hex dumps
- Interactive debugger for encoded types
- Benchmark suite comparing to alternatives
- Integration with serialization frameworks (serde visitor)
