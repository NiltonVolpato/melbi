# Design Doc: Numeric Safety and Arithmetic Error Handling

**Author**: @NiltonVolpato

**Date**: 11-08-2025

## Introduction

### Background

Melbi aims for a "no runtime errors" philosophy through its effect system, but numeric operations present unique challenges:

**Problems to solve:**
1. **Integer overflow/underflow** - Silent wraparound can cause subtle bugs
2. **Float special values** - NaN and Infinity propagate silently in IEEE 754
3. **Lossy casts** - Converting Float→Int truncates without warning
4. **Division by zero** - Critical error that must be caught
5. **Invalid UTF-8** - Bytes→Str cast can fail

Different use cases need different safety levels:
- **Financial calculations** - Need strict checking, overflow is a bug
- **Graphics/game code** - May want saturating arithmetic for colors
- **Performance-critical paths** - Wrapping arithmetic is acceptable
- **General scripting** - Reasonable defaults with opt-in strictness

**Stakeholders:**
- **Melbi users** - Need clear, predictable numeric behavior
- **Embedders** - Need to control safety policies for their domain
- **Type system** - Must track which operations can error
- **Effect system** - Must integrate with `!` effect tracking

### Current Functionality

As of this design:
- **Integer arithmetic** uses wrapping (no overflow checking)
- **Division by zero** produces runtime error (always)
- **Float division by zero** produces Infinity (IEEE 754)
- **Casts** are permissive (NaN→0, Inf→MAX/MIN, truncate)
- **Bytes→Str** can fail at runtime (invalid UTF-8)
- `otherwise` can catch runtime errors

**What works:**
```melbi
(10 / 0) otherwise 42           // Catches DivisionByZero
(bytes as Str) otherwise "bad"  // Catches InvalidUtf8
```

**What's missing:**
- No overflow/underflow detection
- No way to make Float division fail on NaN/Infinity
- No strict casting mode
- No compile-time tracking of which operations can fail (awaits effect system)

### In Scope

This design addresses:

1. **Checked operators** - Explicit overflow/NaN checking (`+!`, `/!`, `as!`)
2. **Arithmetic modes** - Expression-level safety policies via pragmas
3. **Casting modes** - Strict vs permissive casting via pragmas
4. **Type class integration** - Associated effects per numeric type
5. **Effect system integration** - Tracking `!` effect for checked operations
6. **Implementation timeline** - Phased approach before and after effect system

### Out of Scope

- **Arbitrary precision arithmetic** - Stay with i64/f64
- **Modular arithmetic** - No `%` operator safety (future consideration)
- **User-defined numeric types** - Type classes are internal only
- **Dependent types** - No range types like `Int[0..100]` (future extension)
- **Compile-time overflow detection** - Focus on runtime checking
- **Hardware-specific behavior** - Target IEEE 754 and two's complement

### Assumptions & Dependencies

- Effect system will be implemented (tracking `!` in types)
- Type classes (traits) will be implemented internally
- `otherwise` operator already exists for error handling
- Pragma system will be implemented (see `metadata-directives.md`)
- Arena-based allocation for efficient error handling

### Terminology

- **Wrapping arithmetic** - Overflow wraps around (i64::MAX + 1 → i64::MIN)
- **Saturating arithmetic** - Overflow clamps to MAX/MIN
- **Checked arithmetic** - Overflow produces error effect
- **Checked operator** - Operator with `!` suffix (e.g., `+!`, `/!`)
- **Strict cast** - Cast that errors on NaN, Infinity, or non-exact conversion
- **Permissive cast** - Cast that does lossy conversion without error
- **Associated effect** - Effect type associated with a type class operation
- **Type class** - Internal trait-like structure for polymorphism (not user-definable)

## Considerations

### Concerns

**Complexity**: Adding multiple modes increases cognitive load. Must keep defaults sensible and explicit opt-in clear.

**Performance**: Checked arithmetic has overhead. Wrapping should remain default for performance-critical code.

**Consistency**: Behavior must be predictable across Int and Float types, even though they have different failure modes.

**Migration**: Existing code assumes wrapping. Must not break when adding checked modes.

**Type class complexity**: Associated effects are a sophisticated feature. Implementation must be sound.

### Operational Readiness Considerations

Not applicable - Melbi is a language, not a deployed service. However:

**Metrics to consider:**
- Frequency of overflow in user code (instrumentation mode?)
- Performance impact of checked vs wrapping arithmetic
- Adoption rate of checked operators vs pragmas

**Debugging:**
- Clear error messages indicating what overflowed and where
- IDE integration showing which operations can fail
- Good span tracking for overflow errors

### Open Questions

1. **Should saturating mode be in MVP?**
   - **Leaning towards:** Defer until proven need
   - **Reasoning:** Wrapping and checked cover most use cases

2. **Float behavior in checked mode - what counts as "error"?**
   - **Proposal:** Operations producing NaN or Infinity error
   - **Example:** `1.0e308 * 10.0` → error (would be Infinity)
   - **Open:** Should subnormal numbers error? (Probably not)

3. **Should there be separate int/float pragmas?**
   - **Proposal:** Yes, via `%arithmetic.int checked` and `%arithmetic.float checked`
   - **Reasoning:** Some domains want strict int but permissive float

4. **What about compound operations like `+=` (if we add them)?**
   - **Proposal:** Defer - no mutation in MVP
   - **If added:** `x += y` uses mode, `x +!= y` always checks

5. **Should `bytes as Str` have an `as!` variant?**
   - **Leaning towards:** No - it's already fallible, `as!` would be redundant
   - **Alternative:** Better error messages for strict mode

### Cross-Region Considerations

Not applicable - language specification, not deployed infrastructure.

## Proposed Design

### Solution

Implement a three-tier numeric safety system:

1. **Explicit checked operators** (`+!`, `-!`, `*!`, `/!`, `^!`, `as!`)
   - Work immediately, before effect system
   - Always produce errors on overflow/NaN/invalid casts
   - Once effect system exists, produce `!` effect in types

2. **Arithmetic and casting pragmas** (`%arithmetic checked`, `%casts strict`)
   - Require effect system (change types globally)
   - Apply policy to all operations in expression
   - Override type class associated effects

3. **Type class associated effects**
   - Internal compiler structure (not user syntax)
   - Track which numeric operations can fail per type
   - Example: `Int` division errors, `Float` division doesn't

### System Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     User Code                           │
│  - Checked operators: x +! y                            │
│  - Pragmas: %arithmetic checked                         │
└─────────────────┬───────────────────────────────────────┘
                  │
                  v
┌─────────────────────────────────────────────────────────┐
│                  Type Checker                           │
│  - Infer effects from operators                         │
│  - Apply pragma modes to all operations                 │
│  - Query type class associated effects                  │
│  - Produce types with ! effect: Int!, Float!            │
└─────────────────┬───────────────────────────────────────┘
                  │
                  v
┌─────────────────────────────────────────────────────────┐
│                   Evaluator                             │
│  - Use checked_add/checked_mul for +!, *!              │
│  - Detect NaN/Infinity for Float checked ops           │
│  - Validate strict casts                                │
│  - Throw runtime errors on failure                      │
└─────────────────────────────────────────────────────────┘
```

### Data Model

#### Type Class Structure*

**[*] This uses `trait`/`impl` syntax for illustration only. This is an internal compiler structure, not user-facing Melbi syntax.**

```rust
trait Numeric {
    type DivEffect;       // Associated effect for division
    type OverflowEffect;  // Associated effect for overflow (in checked mode)

    fn add(Self, Self) => Self;
    fn sub(Self, Self) => Self;
    fn mul(Self, Self) => Self;
    fn div(Self, Self) => Self with DivEffect;
    fn pow(Self, Self) => Self;
}

impl Numeric for Int {
    type DivEffect = !;       // Division by zero errors
    type OverflowEffect = !;  // Overflow produces errors

    fn div(x, y) =>! Int { x / y }
}

impl Numeric for Float {
    type DivEffect = ∅;       // Division produces Infinity, no error
    type OverflowEffect = ∅;  // Overflow produces Infinity, no error

    fn div(x, y) => Float { x / y }
}
```

#### Arithmetic Modes

```rust
enum ArithmeticMode {
    Wrapping,   // Default: overflow wraps, no error
    Checked,    // All arithmetic ops produce ! on overflow
    Saturating, // (Future) Clamps to MAX/MIN
}

enum CastMode {
    Permissive, // Default: truncate, NaN→0, Inf→MAX/MIN
    Strict,     // All casts produce ! on NaN/Inf/non-exact
}
```

### Interface / API Definitions

#### Checked Operator Syntax (Actual Melbi Code)

```melbi
// Checked arithmetic
x +! y    // Add with overflow check
x -! y    // Subtract with overflow check
x *! y    // Multiply with overflow check
x /! y    // Divide with zero check (and Infinity/NaN check for Float)
x ^! y    // Power with overflow check

// Checked casting
value as! Type    // Strict cast (errors on NaN, Infinity, non-exact)
```

**Precedence:** Same as non-checked counterparts
```melbi
a +! b + c   // = (a +! b) + c
a + b +! c   // = a + (b +! c)
```

#### Pragma Syntax (Actual Melbi Code)

```melbi
%arithmetic wrapping   // Default
%arithmetic checked    // All arithmetic produces ! on overflow

%casts permissive      // Default
%casts strict          // All casts produce ! on NaN/Inf/non-exact

// Future: Separate int/float control
%arithmetic.int checked
%arithmetic.float wrapping
```

### Business Logic

#### Type Checking Integration

**Without effect system (early phases):**
- Checked operators invoke runtime checks
- Types are still `Int`, `Float` (no `!` tracking)
- No compile-time enforcement

**With effect system:**
```rust
fn check_binary(&mut self, op: BinaryOp, left_ty: Type, right_ty: Type) -> Type {
    let base_effects = left_ty.effects.union(right_ty.effects);

    let operation_effects = match (op, self.arithmetic_mode, op.is_checked()) {
        // Division by zero always produces ! (for Int)
        (BinaryOp::Div, _, _) if left_ty.is_int() => EffectSet::ERROR,

        // Checked operators always produce ! (override type class)
        (_, _, true) => EffectSet::ERROR,

        // In checked mode, all arithmetic produces !
        (_, ArithmeticMode::Checked, _) if op.is_arithmetic() => EffectSet::ERROR,

        // Otherwise, query type class for associated effect
        _ => self.query_type_class_effect(op, left_ty.data),
    };

    Type {
        data: result_data,
        effects: base_effects.union(operation_effects),
    }
}

fn query_type_class_effect(&self, op: BinaryOp, ty: DataType) -> EffectSet {
    // Query the type class for this operation's associated effect
    match (ty, op) {
        (DataType::Int, BinaryOp::Div) => EffectSet::ERROR,  // Int.DivEffect = !
        (DataType::Float, BinaryOp::Div) => EffectSet::TOTAL, // Float.DivEffect = ∅
        // ... other combinations
    }
}
```

#### Evaluator Integration

**Checked integer arithmetic:**
```rust
fn eval_binary_int_checked(
    op: BinaryOp,
    left: i64,
    right: i64,
    span: Span
) -> Result<i64, EvalError> {
    match op {
        BinaryOp::Add => left.checked_add(right)
            .ok_or(EvalError::Overflow { span }),
        BinaryOp::Mul => left.checked_mul(right)
            .ok_or(EvalError::Overflow { span }),
        BinaryOp::Div => {
            if right == 0 {
                Err(EvalError::DivisionByZero { span })
            } else {
                left.checked_div(right)
                    .ok_or(EvalError::Overflow { span })
            }
        }
        // ... other ops
    }
}
```

**Checked float arithmetic:**
```rust
fn eval_binary_float_checked(
    op: BinaryOp,
    left: f64,
    right: f64,
    span: Span
) -> Result<f64, EvalError> {
    let result = match op {
        BinaryOp::Add => left + right,
        BinaryOp::Mul => left * right,
        BinaryOp::Div => left / right,
        // ... other ops
    };

    // Check for NaN or Infinity
    if result.is_nan() || result.is_infinite() {
        Err(EvalError::FloatOverflow { span })
    } else {
        Ok(result)
    }
}
```

**Strict casting:**
```rust
fn perform_strict_cast(value: f64, span: Span) -> Result<i64, EvalError> {
    // Check for special values
    if value.is_nan() {
        return Err(EvalError::CastError {
            message: "Cannot cast NaN to Int".into(),
            span
        });
    }

    if value.is_infinite() {
        return Err(EvalError::CastError {
            message: "Cannot cast Infinity to Int".into(),
            span
        });
    }

    // Check for non-exact conversion
    if value.fract() != 0.0 {
        return Err(EvalError::CastError {
            message: format!("Cannot cast {} to Int (not exact)", value),
            span
        });
    }

    // Check for overflow
    if value > i64::MAX as f64 || value < i64::MIN as f64 {
        return Err(EvalError::CastError {
            message: "Float value out of Int range".into(),
            span
        });
    }

    Ok(value as i64)
}
```

### Migration Strategy

This is a new feature, not a migration. Implementation is additive and backwards-compatible:

**Phase 1: Checked Operators (Before Effect System)**
- Add `+!`, `-!`, `*!`, `/!`, `^!`, `as!` to parser
- Implement runtime checking in evaluator
- Types remain `Int`, `Float` (no `!` yet)
- Works with existing `otherwise`

**Phase 2: Effect System**
- Implement `!` effect tracking in type checker
- Checked operators now produce `Int!`, `Float!`
- Top-level `!` requires `otherwise`

**Phase 3: Pragmas (After Effect System)**
- Add `%arithmetic` and `%casts` pragma parsing
- Type checker applies modes to all operations
- Pragmas change type system behavior (add `!` globally)

**Phase 4: Type Class Associated Effects**
- Implement internal type class system
- Query associated effects per operation per type
- Int division always has `!`, Float division doesn't

**Phase 5: (Maybe) Saturating Mode**
- Add `%arithmetic saturating` pragma
- Implement `saturating_add`, etc. in evaluator

**No breaking changes:** Existing code continues to work (wrapping is default).

### Work Required

#### Phase 1: Checked Operators (~3-5 days)
- Parser: Lex and parse `+!`, `-!`, `*!`, `/!`, `^!` (50 lines)
- Parser: Lex and parse `as!` (30 lines)
- AST: Add `checked` flag to BinaryOp and Cast (20 lines)
- Evaluator: Implement checked integer ops (100 lines)
- Evaluator: Implement checked float ops (80 lines)
- Evaluator: Implement strict casting (60 lines)
- Tests: Comprehensive operator tests (200 lines)
- Tests: Strict cast tests (100 lines)

**Total: ~640 lines**

#### Phase 2: Effect System Integration (~2-3 days after effect system exists)
- Type checker: Mark checked ops as producing `!` (50 lines)
- Type checker: Update function type inference (30 lines)
- Tests: Effect propagation tests (100 lines)

**Total: ~180 lines**

#### Phase 3: Pragma Support (~4-6 days after pragma system exists)
- Parser: Already done in `metadata-directives.md`
- Type checker: Apply arithmetic mode to ops (80 lines)
- Type checker: Apply cast mode to casts (40 lines)
- Evaluator: Mode-aware evaluation dispatch (60 lines)
- Tests: Pragma interaction tests (150 lines)

**Total: ~330 lines**

#### Phase 4: Type Class Associated Effects (~5-7 days)
- Type system: Define Numeric trait structure (100 lines)
- Type system: Implement Int/Float instances (80 lines)
- Type checker: Query associated effects (60 lines)
- Type checker: Integrate with effect inference (40 lines)
- Tests: Polymorphic function tests (150 lines)

**Total: ~430 lines**

#### Phase 5: (Maybe) Saturating Mode (~2-3 days if needed)
- Evaluator: Implement saturating ops (60 lines)
- Tests: Saturation behavior tests (80 lines)

**Total: ~140 lines**

**Grand Total: ~1,720 lines** (excluding saturating mode)

### Work Sequence

1. **After `otherwise` exists** → Implement Phase 1 (checked operators)
2. **After effect system exists** → Implement Phase 2 (effect integration)
3. **After pragma system exists** → Implement Phase 3 (pragma modes)
4. **After type classes exist** → Implement Phase 4 (associated effects)
5. **(If needed)** → Implement Phase 5 (saturating mode)

**Dependencies:**
- Phase 1: Independent (can start immediately after `otherwise`)
- Phase 2: Requires effect system
- Phase 3: Requires pragma system + effect system
- Phase 4: Requires type class system + effect system
- Phase 5: Requires Phase 3

### High-level Test Plan

#### Unit Tests

**Checked operators:**
```melbi
// Overflow detection
9223372036854775807 +! 1  // Error: Overflow
(9223372036854775807 +! 1) otherwise 0  // Returns: 0

// Float NaN/Infinity detection
(1.0e308 *! 10.0) otherwise 0.0  // Error caught
(0.0 /! 0.0) otherwise 1.0       // NaN caught
```

**Strict casts:**
```melbi
(3.14 as! Int) otherwise 0     // Error: not exact
(3.0 as! Int)                  // OK: 3
(NaN as! Int) otherwise 0      // Error: NaN
```

**Pragma modes:**
```melbi
%arithmetic checked
(100 + 200)                    // Type: Int!
((100 + 200) otherwise 0)      // Type: Int

%casts strict
(value as Int)                 // Type: Int!
```

#### Integration Tests

**Mixed modes:**
```melbi
%arithmetic wrapping
x + y +! z                     // Only z addition checked
```

**Type class polymorphism:**
```melbi
divide = (x, y) => x / y

divide(10, 2)                  // Type: Int! (division can error)
divide(10.0, 0.0)              // Type: Float (produces Infinity)
```

**Function effect inference:**
```melbi
risky = (x, y) => x +! y
// Inferred type: (Int, Int) =>! Int

(risky(100, 200)) otherwise 0  // Must handle
```

#### Property-Based Tests

- Checked operations never panic (always return Error or Ok)
- `(x +! y) otherwise z` has same type as `z`
- In checked mode, all arithmetic has `!` effect
- Wrapping mode never produces `!` (except division)
- Strict casts always produce `!` effect

### Deployment Sequence

Not applicable - this is a language feature, not a deployed service.

**Release sequence:**
1. Release with Phase 1 (checked operators)
   - Document in changelog
   - Show examples in docs
   - Note: Types don't show `!` yet

2. Release with Phase 2 (effect integration)
   - Breaking: Unhandled checked ops now compile errors
   - Migration: Add `otherwise` where needed
   - Benefits: IDE shows which ops can fail

3. Release with Phase 3 (pragmas)
   - New capability: Expression-wide modes
   - Document use cases (financial, graphics, etc.)

4. Release with Phase 4 (type classes)
   - Performance improvement: Polymorphic code
   - Better error messages: Show which type caused error

## Impact

### Performance

**Checked arithmetic overhead:**
- ~5-10% slower than wrapping (due to overflow checks)
- Negligible for most embedded use cases
- Hot paths can use wrapping mode explicitly

**Optimization opportunities:**
- Compiler can elide checks when values are known small
- LLVM can optimize `checked_add` in many cases
- Type class monomorphization enables specialization

**Measurements needed:**
- Benchmark wrapping vs checked arithmetic
- Profile real-world expressions with different modes
- Measure effect of pragma overhead

### Security

**Positive impacts:**
- Overflow bugs become explicit errors (not silent wraparound)
- Financial calculations can enforce strictness
- Sandboxing: Can disallow unchecked arithmetic via pragma enforcement

**No negative impacts:**
- Wrapping remains default (no surprise breakage)
- Users opt into strictness explicitly

### Usability

**Major improvements:**
- Users choose safety level appropriate to their domain
- Clear distinction between checked and unchecked operations
- IDE integration shows which operations can fail
- Good error messages explain what overflowed

**Learning curve:**
- Default behavior (wrapping) is familiar
- Checked operators explicit and googleable
- Pragmas for when you need strictness everywhere

### Cost Analysis

**Development cost:**
- ~15-25 days of engineering across all phases
- Most complexity in type class integration

**Runtime cost:**
- Negligible (errors on cold path)
- Checked arithmetic ~5-10% slower than wrapping
- Users can choose based on needs

**Maintenance cost:**
- Low - design is clean and orthogonal
- Effect system handles propagation automatically
- Type classes are internal (no user-facing API)

## Alternatives

### Alternative 1: Always Check (Rejected)

Make all arithmetic checked by default, require `+~` for wrapping.

**Pros:**
- Safest option
- Forces users to think about overflow

**Cons:**
- Performance hit on all arithmetic
- Verbose for majority use case
- Breaking change from typical language expectations

**Why rejected:** Too strict for default, hurts performance.

### Alternative 2: Saturating by Default (Rejected)

Make all arithmetic saturate to MAX/MIN by default.

**Pros:**
- Intuitive for many use cases (especially UI)
- No silent wraparound

**Cons:**
- Still wrong for financial code (need explicit error)
- Performance cost
- Unusual default (not what most languages do)

**Why rejected:** Wrong default for most domains.

### Alternative 3: No Modes, Only Explicit Operators (Rejected)

Only support `+!`, no pragmas.

**Pros:**
- Simpler implementation
- Explicit at call site

**Cons:**
- Verbose for strictly-checked expressions
- Can't enforce policy (e.g., "all arithmetic checked in this module")

**Why rejected:** Pragmas enable better code organization.

### Alternative 4: Runtime Mode Switching (Rejected)

Let users set arithmetic mode at runtime via host API.

**Pros:**
- Flexible per-evaluation

**Cons:**
- Types can't reflect runtime mode (breaks effect system)
- Confusing semantics
- Performance overhead

**Why rejected:** Mode must be compile-time for type safety.

### Alternative 5: No Float Checking (Rejected)

Only check integer operations, leave floats as IEEE 754.

**Pros:**
- Simpler (fewer cases to handle)
- Float NaN/Infinity is standard behavior

**Cons:**
- Inconsistent (why check Int but not Float?)
- Some domains need strict Float (safety-critical systems)

**Why rejected:** Consistency matters, users can opt out if they want IEEE 754.

## Looking into the Future

### Potential Extensions

**1. Arbitrary Precision Integers**
```melbi
%experimental bigint
let huge = 2 ^ 1000  // No overflow, arbitrary size
```

**2. Range Types**
```melbi
let age: Int[0..120] = user_input as Int
// Bounds checked at cast time, optimized thereafter
```

**3. Units of Measurement**
```melbi
let speed = 100`m/s`
let time = 10`s`
let distance = speed * time  // Type: Quantity[m, 1]
```

**4. Formal Verification Integration**
```melbi
%arithmetic checked
%verify no-overflow
expression  // Proves statically that no overflow occurs
```

**5. Saturation with Bounds**
```melbi
%arithmetic saturating[0, 255]
color.red + adjustment  // Saturates to [0, 255]
```

**6. Per-Function Arithmetic Modes**
```melbi
risky = (x, y) %arithmetic checked => x * y + 1
// Function body uses checked mode, caller doesn't
```

### Next Steps

1. **Implement Phase 1** (checked operators)
   - Validate runtime behavior
   - Gather user feedback on syntax
   - Measure performance overhead

2. **Implement effect system** (prerequisite for later phases)

3. **Implement Phase 2** (effect integration)
   - Validate type checking with `!`
   - Test IDE integration
   - Ensure good error messages

4. **Implement pragma system** (see `metadata-directives.md`)

5. **Implement Phase 3** (pragma modes)
   - Validate expression-wide policies
   - Test interaction with checked operators

6. **Implement type classes** (prerequisite for Phase 4)

7. **Implement Phase 4** (associated effects)
   - Complete polymorphic numeric operations
   - Validate effect inference

8. **(If needed) Implement Phase 5** (saturating mode)

### Nice to Haves

- **Overflow analysis pass** - Warn about potential overflow at compile time
- **Range inference** - Track value ranges to elide runtime checks
- **LLVM intrinsics** - Use hardware overflow detection when available
- **Compile-time evaluation** - Constant-fold checked arithmetic
- **Better error messages** - "Value X overflowed when adding Y"
- **Benchmarking mode** - Instrument to find overflow hot spots

---

**Document Status**: Design phase
**Next Action**: Wait for `otherwise` operator, then implement Phase 1 (checked operators)
**Related Docs**: `effects.md`, `metadata-directives.md`, `error-handling.md`
