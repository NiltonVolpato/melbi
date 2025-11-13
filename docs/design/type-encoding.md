---
title: Melbi Type Encoding Format - Technical Design Document
---

# Design Doc: Melbi Type Encoding Format

**Author**: @NiltonVolpato

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

### Design Goals

This format is optimized for:

1. **Lazy Navigation**: Access any part of the type hierarchy without decoding the whole structure
   - Jump directly to return type without scanning parameters
   - Skip subtrees using size prefixes
   - Zero-copy iteration over collections

2. **Efficient Encoding/Decoding**:
   - Common types pack into single byte (primitives, simple arrays/maps)
   - Deterministic canonical encoding enables byte-level equality
   - Minimal overhead for nested structures

3. **Self-Describing Format**:
   - No external schema required
   - Size prefixes enable bounds checking
   - Clear discriminant layout for fast type identification

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

#### Format Design Principles

The encoding follows three core principles for efficient navigation:

**Principle 1: Singleton-Before-Array**

When a composite type contains both a singleton element and a sequence, the singleton comes first.

- **Rationale**: Enables O(1) access to the most commonly-accessed element
- **Example**: Function return type before parameter list
- **Benefit**: No need to scan/count array elements to reach the singleton

**Principle 2: Count-Prefixes-Sequence**

Every array or sequence is immediately preceded by its element count (varint).

- **Rationale**: Iterator construction is a single read operation
- **Pattern**: `[varint:count][elem₁][elem₂]...[elemₙ]`
- **Benefit**: Know iteration bounds upfront without scanning

**Principle 3: Size-Prefixed-Composites**

Composite types (with children) include a u16 size field after the discriminant.

- **Rationale**: Enables skipping entire subtrees without parsing
- **Pattern**: `[disc:u8][size:u16_le][payload:size_bytes]`
- **Benefit**: Fast structural comparison and navigation

#### Encoding Layouts

**Primitive Types** (1 byte):
```
[discriminant:u8]
```
Examples: Int (32), Float (33), Array[Int] (96), Map[Str,Bool] (118)

**Composite Types** (3+ bytes):
```
[discriminant:u8][payload_size:u16_le][payload...]
```

**Composite Payload Structures**:

- **Array**: `[element_type_encoding]`
  - Size field covers entire element encoding
  - Supports nested arrays naturally

- **Map**: `[key_type_encoding][value_type_encoding]`
  - Both types encoded sequentially
  - Size field covers both encodings

- **Record**: `[count:varint]([name_len:varint][name:utf8][type_encoding])*`
  - Count enables iterator setup
  - Field name and type paired together

- **Function**: `[return_type_encoding][param_count:varint]([param_type_encoding])*`
  - Return type FIRST (singleton-before-array principle)
  - Then param count + param types
  - Enables O(1) return type access

- **Symbol**: `[part_count:varint]([part_len:varint][part:utf8])*`
  - Count enables iterator setup
  - Each part is length-prefixed string

- **TypeVar (non-packed)**: `[id:u16_le]`
  - Simple u16 identifier

**Key Properties**:
- No overlapping slices (each type owns its byte range)
- Canonical encoding (same structure → same bytes)
- Bounded size per node (u16 max ~64KB)
- Composable (deeply nested types work naturally)

**Varint encoding**: Standard LEB128 (7 bits per byte, high bit = continuation)

### Interface / API Definitions

The API provides three layers of access:

**Layer 1: Direct Encoding**
- `encode(ty: &Type) -> SmallVec<[u8; 16]>` - Encode type to bytes
- Deterministic, canonical output
- Most types fit in 16 bytes (stack-allocated)

**Layer 2: Zero-Copy Navigation (TypeView)**
- `TypeView<'a>` - Wrapper around `&'a [u8]`
- Methods for type-specific access: `as_array()`, `as_map()`, `as_record()`, `as_function()`, `as_symbol()`, `as_typevar()`
- Returns views/iterators, no allocation
- Lazy - only parse what you access
- `as_function()` returns `(TypeView<'a>, ParamsIter<'a>)` for O(1) return type access

**Layer 3: Full Decoding**
- `decode(bytes: &[u8], mgr: &TypeManager) -> Result<&Type, DecodeError>`
- Validates encoding and reconstructs arena type
- Interns result for O(1) equality
- Comprehensive error reporting

**Design Rationale**:
- **Layer 1** for serialization/transmission
- **Layer 2** for pattern matching and navigation (most common)
- **Layer 3** when you need the full Type for inference/unification

See `core/src/types/encoding.rs` for complete API signatures and documentation.

#### Iterator Design

All collections (Record, Function params, Symbol) expose iterators that:
- Read their count on construction via `.new(payload)` (single varint read)
- Validate payload consumption on completion
- Return `Result` items for error handling
- Implement `ExactSizeIterator` when possible

This pattern makes count-prefixing transparent to callers.

#### Error Handling

Decoding returns `Result<T, DecodeError>` with specific error variants:
- `Truncated` - buffer too short
- `SizeMismatch` - size field doesn't match content
- `UnknownDiscriminant` - invalid type discriminant
- `InvalidUtf8` - malformed string data
- `InvalidVarint` - bad varint encoding
- `TrailingBytes` - extra data after type
- `TooDeep` - recursion limit exceeded

All errors include byte offsets for debugging.

See `core/src/types/encoding.rs` for complete error definitions.

### Business Logic

#### Encoding Strategy

The encoder uses a **single-pass recursive descent** with size backpatching:

1. Write discriminant byte
2. For composites: write placeholder size bytes (u16)
3. Recursively encode children
4. Backpatch size with actual payload length

**Optimizations**:
- Try packed encodings first (single byte)
- Use helper `encode_composite()` to eliminate duplication
- SmallVec avoids heap allocation for common types

See `encode_inner()` in `core/src/types/encoding.rs` for implementation details.

#### Decoding Strategy

The decoder performs **lazy validation** during navigation:

1. Check discriminant is valid
2. For composites: validate size field vs actual content
3. Recursively decode children on demand
4. Intern result through TypeManager

**Validation Approach**:
- Size fields validated lazily (only when accessed)
- Iterators validate full consumption on completion
- Enables skipping subtrees without validating them

**Validation Helpers**:
- `validate_composite_size()` - size field checking for Array, Map, Function, Symbol
- `validate_payload_is_empty()` - iterator completion checking

See `decode_from_view()` in `core/src/types/encoding.rs` for implementation details.

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

#### Encoding Performance

**Single-pass with minimal overhead**:
- Primitives: ~1ns (single byte write)
- Packed types (Array[Int], Map[Str,Bool], etc.): ~1ns (single byte)
- Complex types: ~100ns (allocation + recursive encoding)
- Most types fit in 16 bytes (no heap allocation with SmallVec)

**Deterministic and cacheable**:
- Same structure always produces identical bytes
- Enables reliable byte-level equality checks
- Results can be cached across sessions

#### Navigation Performance (Key Design Win)

**Zero-copy access via TypeView**:
- No allocation for any navigation operation
- No parsing until you access specific parts
- Can skip entire subtrees using size prefixes

**Operation complexity**:
- **O(1)**: Discriminant check, category detection, primitive access
- **O(1)**: Array element type, Map key/value types
- **O(1)**: Function return type (singleton-before-array principle)
- **O(n)**: Record field lookup (must scan n fields)
- **O(n)**: Function param iteration (must visit n params)
- **O(n)**: Symbol part iteration (must visit n parts)

**Design principle impact**:
The singleton-before-array principle enables O(1) function return type access without scanning parameters. This is critical for type checking, where return types are accessed far more frequently than individual parameters.

**Example**: Checking if `foo : (Int, Int, Int, Int) -> Bool` returns Bool:
```rust
if let Some((ret, _params)) = view.as_function() {
    if ret.discriminant() == DISC_BOOL {
        // Instant check, never touched the 4 parameters
    }
}
```

#### Equality Performance

**Byte-level comparison**:
- Simple memcmp, exits on first difference
- Typical: ~2-10ns depending on type size
- Still 3-5x slower than pointer equality but acceptable
- Fast path for size mismatch (compare lengths first)

**When to use**:
- Comparing types from different `TypeManager` instances
- Checking if cached type matches incoming type
- Serialized type deduplication

#### Decoding Performance

**Full reconstruction**:
- With validation: ~200ns for complex types
- Validation is lazy - only checks accessed parts
- Interning means decoded types reuse existing instances
- Recursion depth limited (default: 100 levels)

**Memory allocation**:
- Vec allocations for Record fields, Function params, Symbol parts
- Arena allocation for final Type instance
- No intermediate allocations during validation

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
