# Future Projects

This document tracks larger design ideas and features we want to do - some sooner, some later.

**Priorities**: P0 (critical) → P1 (high) → P2 (medium) → P3 (low)  
_Re-evaluate priorities periodically as needs change_

---

## Numeric Safety and Arithmetic Error Handling

**Priority**: P1 (High)  
**Status**: Design complete, awaiting implementation  
**Design Doc**: [numeric-safety.md](design/numeric-safety.md)

**Context**: Currently, Melbi uses wrapping semantics for arithmetic overflow and lossy type conversions to avoid runtime errors.

**Proposal**: Add a "strict mode" (possibly via metadata directives like `@strict`) where:

1. **Wrapping Arithmetic → Error**
   - Integer overflow/underflow returns an error instead of wrapping
   - Example: `i64::MAX + 1` would fail instead of wrapping to `i64::MIN`
   - Users can handle with `otherwise`: `(x + 1) otherwise fallback`

2. **Lossy Casts → Error**
   - `Float → Int` fails if the float value cannot be exactly represented as an integer
   - Example: `3.7 as Int` would fail (not exactly representable)
   - Example: `f64::INFINITY as Int` would fail
   - Example: `f64::NAN as Int` would fail
   - But: `3.0 as Int` succeeds (exact representation)

3. **Benefits**:
   - Users who want stricter guarantees can opt-in
   - Default behavior remains permissive (no surprises for casual users)
   - Aligns with Melbi's "no runtime errors if it type-checks" goal in strict mode

4. **Design Questions**:
   - How to enable strict mode? Global flag? Per-expression annotation?
   - Should there be gradations? (e.g., strict arithmetic but permissive casts?)
   - How does this interact with the effect system?

**Related Files**:
- `core/src/evaluator/operators.rs` - Arithmetic operators use `wrapping_*` methods
- `core/src/casting.rs` - Cast implementations
- `docs/design/metadata-directives.md` - Potential syntax for annotations

**References**:
- See [evaluator-implementation-plan.md](evaluator-implementation-plan.md) Q1 (Arithmetic Overflow Behavior)
- See [lambda-closure-implementation-plan.md](lambda-closure-implementation-plan.md) for effect system integration

---

## Type Classes / Traits

**Priority**: P1 (High)  
**Status**: Planning phase - needed for Maps and numeric safety

**Context**: Currently Melbi only supports universal quantification (parametric polymorphism). Many useful abstractions require constraints on type variables.

**Examples Requiring Type Classes**:

1. **Hashable constraint for Maps**
   - Maps need keys to be hashable
   - Type signature: `Map[K, V] where K: Hashable`
   - Without this, maps would need to restrict keys to specific types

2. **Ord constraint for sorted operations**
   - Sorting requires comparable elements
   - Type signature: `sort[T](Array[T]) -> Array[T] where T: Ord`

3. **Num constraint for numeric operations**
   - Generic math functions need numeric operations
   - Example: `(a) => a + a` would require `a: Num` (see lambda plan)

**Design Questions**:
- Haskell-style type classes vs Rust-style traits?
- Implicit vs explicit constraint syntax?
- How to define type class instances for built-in types?
- FFI functions with type class constraints?

**Related Files**:
- `docs/lambda-closure-implementation-plan.md` - Q1 discusses polymorphic operators
- `core/src/types/` - Type system implementation

---

## Additional Future Ideas

_(This section can grow as we discover more patterns during implementation)_

### Recursive Functions

**Priority**: P2 (Medium)
- Currently explicitly disallowed for closures
- Could revisit if there's a safe, bounded approach
- See `docs/lambda-closure-implementation-plan.md` Phase 4.1

### Tail Call Optimization

**Priority**: P3 (Low)
- Would enable safe recursion without stack overflow
- Requires recognizing tail position calls
- Lower priority until recursion is supported

### Pattern Matching

**Priority**: P2 (Medium)
- Currently only used implicitly in type system (union types)
- Explicit `match` expressions could be useful
- Would need exhaustiveness checking

### Const Evaluation / Compile-Time Execution

**Priority**: P3 (Low)
- Evaluate pure expressions at compile time
- Requires effect system to identify pure expressions
- See `docs/design/effects.md`

---

## Notes

This document is intentionally informal and exploratory. Ideas here are:
- Not committed to
- Subject to change based on real-world usage
- May be superseded by better designs

When an idea is ready for implementation, move it to a proper design document or implementation plan.
