# Design Doc: Effect System in Melbi

**Author**: @NiltonVolpato

**Date**: 10-20-2025

## Introduction

### Background

Melbi is an embeddable expression language designed for safe evaluation of user
code. To achieve its goals of "no runtime errors" and optimal performance
through constant folding, we need a type-level effect system that tracks:

1. **Computations that can fail** - to force error handling and prevent crashes
2. **Computations that depend on runtime context** - to enable aggressive
   compile-time optimization

Without an effect system, we cannot guarantee safety (unhandled errors crash) or
optimal performance (missing constant folding opportunities).

### Current Functionality

Currently (as of this design doc), Melbi has:

- Basic type system with `DataType` enum representing type structure
- `Type` struct wrapping `DataType` with an `EffectSet`
- `EffectSet` with two boolean flags: `can_error` and `is_impure`

Effects are not yet implemented in the type checker, evaluator, or optimizer.

### In Scope

This design covers:

- Formal semantics of the two effects: `!` (error) and `~` (impure)
- Effect inference and propagation rules
- Integration with type checking, evaluation, and optimization
- Effect handling via `otherwise` operator
- FFI function effect declarations

### Out of Scope

- Effect handlers (algebraic effects) - future extension
- Region-based effects (like Rust lifetimes) - not needed for Melbi's use case
- User-visible effect annotations - effects are inferred automatically
- Multiple error types - start with single `!`, extend later if needed

### Assumptions & Dependencies

- Type system is implemented with `DataType` and `Type` separation
- Evaluator/VM will be implemented to identify error sources
- Constant folding optimizer will be implemented
- Arena-based allocation is used throughout

### Terminology

- **Effect**: A property of a computation tracked by the type system
- **`!` (error effect)**: Indicates a computation that may fail at runtime
- **`~` (impure effect)**: Indicates a computation that depends on runtime
  context (inputs, I/O, non-determinism)
- **`¬` (never effect)**: Internal marker indicating a computation that never
  executes (short-circuits due to error)
- **Effect propagation**: How effects flow from sub-expressions to containing
  expressions
- **Constant folding**: Compile-time evaluation of pure expressions

## Considerations

### Concerns

**Complexity**: Effect systems can become complex quickly. We must keep the
design minimal and pragmatic.

**Implementation phases**: Effects should be added incrementally (evaluator
first, then optimizer) to validate the design at each step.

**User experience**: Most users should never need to think about effects. Only
FFI authors need to understand them.

**Performance**: Effect tracking should have negligible runtime overhead since
it's all compile-time.

### Open Questions

1. Should `¬` (never effect) be exposed in error messages, or kept internal?

   - **Answer**: Keep internal, only show `!` to users

2. How do we handle functions that are sometimes total based on their arguments?
   (e.g., `divide(x, 0)` always errors, but `divide(x, constant_nonzero)`
   doesn't)

   - **Answer**: Conservative - track effects pessimistically, optimize later
     with constant propagation

3. Should we track which specific parameters contribute effects, or use
   conservative "any parameter used" approach?

   - **Answer**: Start conservative, add per-parameter tracking if needed

### Operational Readiness Considerations

**Metrics**: Track in type checker:

- % of expressions that are pure (can be constant folded)
- % of error effects that are handled at appropriate levels

**Debugging**: Effect-related type errors should include:

- Which expression introduced the effect
- Suggestions for how to handle it (`otherwise`, pattern matching)
- Context showing the effect propagation chain

## Proposed Design

### Solution

Implement a lightweight effect system with two effects (`!` for errors, `~` for
impurity) that:

1. Prevents unhandled errors from reaching runtime
2. Enables aggressive constant folding of pure expressions
3. Is completely invisible to most users
4. Requires minimal implementation complexity

### System Architecture

Effects integrate into three major components:

```
┌─────────────────┐
│  Type Checker   │ ← Infers and propagates effects
└────────┬────────┘
         │
         v
┌─────────────────┐
│  Evaluator/VM   │ ← Identifies error sources, validates effect tracking
└────────┬────────┘
         │
         v
┌─────────────────┐
│   Optimizer     │ ← Constant folds pure expressions (no ~)
└─────────────────┘
```

### Data Model

```rust
/// Effects tracked by the type system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectSet {
    /// Whether the computation can result in an error (!)
    ///
    /// Examples: division by zero, array out of bounds, map key not found
    pub can_error: bool,

    /// Whether the computation depends on runtime context (~)
    ///
    /// This includes:
    /// - Using input data passed from the host
    /// - Performing I/O operations
    /// - Non-deterministic operations (random, timestamps)
    pub is_impure: bool,
}

impl EffectSet {
    /// No effects - pure, total computation
    pub const TOTAL: Self = EffectSet {
        can_error: false,
        is_impure: false
    };

    /// Can fail, but otherwise pure
    pub const ERROR: Self = EffectSet {
        can_error: true,
        is_impure: false
    };

    /// Impure but cannot fail
    pub const IMPURE: Self = EffectSet {
        can_error: false,
        is_impure: true
    };

    /// Both impure and can fail
    pub const BOTH: Self = EffectSet {
        can_error: true,
        is_impure: true
    };

    /// Union of two effect sets (for propagation)
    pub fn union(self, other: Self) -> Self {
        EffectSet {
            can_error: self.can_error || other.can_error,
            is_impure: self.is_impure || other.is_impure,
        }
    }

    pub fn is_total(&self) -> bool {
        *self == Self::TOTAL
    }
}

/// The type of an expression, including its effects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Type<'a> {
    /// The structure of the type (Int, Array, Function, etc.)
    pub data: &'a DataType<'a>,

    /// The effects of computing this type
    pub effects: EffectSet,
}

impl<'a> Type<'a> {
    /// Add effects to this type
    pub fn with_effects(self, effects: EffectSet) -> Self {
        Type {
            data: self.data,
            effects: self.effects.union(effects),
        }
    }

    /// Check if this is a pure, total computation
    pub fn is_total(&self) -> bool {
        self.effects.is_total()
    }
}
```

### Effect Semantics

#### The `!` (Error) Effect

**Meaning**: This computation may fail at runtime with an error.

**Sources of `!`**:

- Division: `x / y` (division by zero)
- Array indexing: `arr[i]` (out of bounds)
- Map lookup: `map[key]` (key not found)
- Explicit error: `error("message")`

**Propagation**: Automatically propagates through all operations:

```melbi
(a / b) + (c / d)  // Int! - error from either division propagates
```

**Handling**: Must be handled before top-level:

```melbi
// ❌ ERROR: Unhandled error effect
10 / x

// ✅ OK: Error handled with otherwise
(10 / x) otherwise 0

// ✅ OK: Error handled with pattern matching (future)
(10 / x) otherwise match {
    Ok(n) -> n,
    DivisionByZero -> 0,
}
```

**Key property**: `!` can be removed by handling, but only explicitly.

#### The `~` (Impure) Effect

**Meaning**: This computation depends on external context and cannot be
constant-folded.

**Sources of `~`**:

- Input data from host: Any variable passed into the expression
- I/O operations: `print()`, `read_file()`, etc.
- Non-determinism: `random()`, `now()`, etc.

**Propagation**: Automatically propagates through all operations:

```melbi
email.sender ++ " is blocked"  // String~ - impurity propagates
```

**Key property**: `~` is indelible - it cannot be removed. Once a computation
depends on runtime context, it always will.

**Purpose**: Enables constant folding:

- Expressions without `~` → evaluated at compile time
- Expressions with `~` → evaluated at runtime

**Example**:

```melbi
// Email filter expression
email.sender not in {"spam@example.com": true, "bad@actor.com": true, ...}

// Type analysis:
// - email.sender: String~ (uses input)
// - The map literal: Map[String, Bool] (pure!)
// - Result: Bool~

// Optimization:
// The map is constant-folded at compile time
// Only the membership test runs at runtime
```

#### The `¬` (Never) Effect (Internal Only)

**Meaning**: This computation short-circuits due to an error in its arguments and does not execute.

**When it appears**: When a function is called with an erroring argument:
```melbi
Foo(1 / 0)
// 1 / 0 has type Int!
// Foo's body short-circuits (doesn't execute)
// But the error still propagates → result type is String!, not String¬
```

**Important clarification**: The `¬` effect is NOT about the result type. The function body doesn't execute, but the overall expression still has type `String!` (error propagates). The `¬` marker is purely an internal optimization hint that the function body won't run.

**Key property**: `¬` is an implementation detail for optimization, not a user-facing effect.

**Visibility**: Never shown to users, only tracked internally in the compiler for optimization purposes.

### Effect Inference Rules

#### Binary Operations

```
e1: T1 eff1    e2: T2 eff2
────────────────────────────
e1 ⊕ e2: T3 (eff1 ∪ eff2)
```

Effects from both operands are unioned.

Special case for division:
```
e1: Int eff1    e2: Int eff2
──────────────────────────────────────
e1 / e2: Int (eff1 ∪ eff2 ∪ !)
```

Integer division always adds `!` effect (division by zero).

**Note on type classes**: Float division behaves differently — `Float / Float` produces `Float` (no `!` effect), following IEEE 754 which returns Infinity rather than erroring. This difference is handled through type class associated effects. See `numeric-safety.md` for details on how division behavior varies by numeric type.

#### Arrays

```
e1: T eff1    e2: T eff2    ...    en: T effn
──────────────────────────────────────────────
[e1, e2, ..., en]: Array[T] (eff1 ∪ eff2 ∪ ... ∪ effn)
```

Array construction unions all element effects, but effects bubble out:
`Array[Int!]` becomes `Array[Int]!` (canonicalization).

#### Array Indexing

```
arr: Array[T] eff_arr    idx: Int eff_idx
───────────────────────────────────────────────
arr[idx]: T (eff_arr ∪ eff_idx ∪ !)
```

Indexing always adds `!` effect (out of bounds).

#### Records

```
f1: T1 eff1    f2: T2 eff2    ...    fn: Tn effn
────────────────────────────────────────────────────
{f1 = e1, f2 = e2, ..., fn = en}: Record[...] (eff1 ∪ eff2 ∪ ... ∪ effn)
```

#### Field Access

```
record: Record[f: T] eff_rec
─────────────────────────────────
record.f: T (eff_rec ∪ !)
```

Field access adds `!` (field might not exist for union types).

#### If-Then-Else

```
cond: Bool eff_c    then_branch: T eff_t    else_branch: T eff_e
───────────────────────────────────────────────────────────────────
if cond then then_branch else else_branch: T (eff_c ∪ eff_t ∪ eff_e)
```

All three branches contribute effects.

#### Lambdas

```
body: T_ret eff_body    (inferred by analyzing body)
params_used: Set<ParamName>   (which params are referenced)
─────────────────────────────────────────────────────────
(param1, param2, ...) => body:
    Function where intrinsic effects = eff_body
    and param effects depend on which params are used
```

#### Function Calls

**Normal case** (no erroring arguments):

```
func: (T1, T2, ..., Tn) -> Tret with intrinsic effects eff_func
arg1: T1 eff1    arg2: T2 eff2    ...    argn: Tn effn
──────────────────────────────────────────────────────────────
func(arg1, arg2, ..., argn): Tret (eff_func ∪ eff1 ∪ eff2 ∪ ... ∪ effn)
```

**Short-circuit case** (any argument has `!`):
```
func: (T1, T2, ..., Tn) -> Tret with intrinsic effects eff_func
argi: Ti !    (some argument has error effect)
────────────────────────────────────────────────────
func(..., argi, ...): Tret !
```

When any argument errors, the function body doesn't execute (short-circuits), but the error effect still propagates to the result. The `¬` marker is tracked internally for optimization but doesn't affect the result type.

#### Otherwise (Error Handling)

```
expr: T !    default: T eff_default
────────────────────────────────────
expr otherwise default: T eff_default
```

The `!` effect is removed, replaced with effects from default. The `~` effect
(if present) is preserved.

#### Input Variables

All variables passed from the host automatically have `~`:

```
input_var: T~
```

This marks them as runtime-dependent, preventing constant folding.

### Business Logic

#### Type Checking Integration

During type checking, track effects alongside types:

```rust
fn check_expr(&mut self, expr: &Expr<'a>) -> Type<'a> {
    match expr {
        Expr::Literal(_) => {
            // Literals are pure
            Type { data: ..., effects: EffectSet::TOTAL }
        }

        Expr::Binary { op, left, right } => {
            let left_ty = self.check_expr(left);
            let right_ty = self.check_expr(right);

            // Unify data types
            let result_data = self.unify_data(left_ty.data, right_ty.data)?;

            // Union effects
            let mut effects = left_ty.effects.union(right_ty.effects);

            // Division adds error effect
            if op == BinOp::Div {
                effects = effects.union(EffectSet::ERROR);
            }

            Type { data: result_data, effects }
        }

        Expr::Array(elements) => {
            let elem_types: Vec<_> = elements.iter()
                .map(|e| self.check_expr(e))
                .collect();

            // Unify all element types
            let elem_data = self.unify_all_data(&elem_types)?;

            // Union all effects
            let effects = elem_types.iter()
                .fold(EffectSet::TOTAL, |acc, ty| acc.union(ty.effects));

            // Create array type
            Type {
                data: self.type_manager.intern(DataType::Array(elem_data)),
                effects,
            }
        }

        Expr::Index { array, index } => {
            let array_ty = self.check_expr(array);
            let index_ty = self.check_expr(index);

            // Check array is actually an array
            let elem_data = match array_ty.data {
                DataType::Array(elem) => elem,
                _ => return Err(TypeError::NotAnArray),
            };

            // Check index is an integer
            self.unify_data(index_ty.data, self.type_manager.int_data())?;

            // Union effects and add error effect for out of bounds
            let effects = array_ty.effects
                .union(index_ty.effects)
                .union(EffectSet::ERROR);

            Type { data: elem_data, effects }
        }

        Expr::Otherwise { expr, default } => {
            let expr_ty = self.check_expr(expr);
            let default_ty = self.check_expr(default);

            // Unify data types
            let result_data = self.unify_data(expr_ty.data, default_ty.data)?;

            // Otherwise removes !, but keeps ~
            let effects = EffectSet {
                can_error: false,  // ! is removed
                is_impure: expr_ty.effects.is_impure || default_ty.effects.is_impure,
            };

            Type { data: result_data, effects }
        }

        Expr::Variable(name) => {
            let var_ty = self.env.get(name)?;

            // Variables from input have ~ effect
            if self.is_input_variable(name) {
                var_ty.with_effects(EffectSet::IMPURE)
            } else {
                var_ty
            }
        }

        // ... other expression types
    }
}

/// Check that top-level expression doesn't have unhandled errors
fn check_program(&mut self, expr: &Expr<'a>) -> Result<Type<'a>> {
    let ty = self.check_expr(expr);

    if ty.effects.can_error {
        return Err(TypeError::UnhandledError {
            span: self.span_of(expr),
            suggestion: "Use 'otherwise' or pattern matching to handle errors",
        });
    }

    // ~ is allowed at top level (impure expressions are fine)
    Ok(ty)
}
```

#### Evaluator Integration (Phase 2)

The evaluator validates effect tracking:

```rust
fn eval(&mut self, expr: &Expr<'a>, ty: Type<'a>) -> Result<Value<'a>> {
    match expr {
        Expr::Binary { op: BinOp::Div, left, right } => {
            let left_val = self.eval(left, left_ty)?;
            let right_val = self.eval(right, right_ty)?;

            let divisor = right_val.as_int()?;
            if divisor == 0 {
                // This should only happen if type checking didn't require
                // error handling (! effect)
                debug_assert!(ty.effects.can_error);
                return Err(EvalError::DivisionByZero);
            }

            Ok(Value::Int(left_val.as_int()? / divisor))
        }

        Expr::Index { array, index } => {
            let array_val = self.eval(array, array_ty)?;
            let index_val = self.eval(index, index_ty)?;

            let idx = index_val.as_int()?;
            let arr = array_val.as_array()?;

            if idx < 0 || idx >= arr.len() as i64 {
                // This should only happen if type checking didn't require
                // error handling (! effect)
                debug_assert!(ty.effects.can_error);
                return Err(EvalError::IndexOutOfBounds);
            }

            Ok(arr[idx as usize].clone())
        }

        // ... other cases
    }
}
```

#### Constant Folding Integration (Phase 3)

The optimizer constant-folds pure expressions:

```rust
fn constant_fold(&mut self, expr: &Expr<'a>, ty: Type<'a>) -> Option<Value<'a>> {
    // Only fold expressions without impure effect
    if ty.effects.is_impure {
        return None;
    }

    // For pure expressions, try to evaluate at compile time
    match self.try_eval_pure(expr) {
        Ok(value) => {
            // Success! Replace expression with constant
            Some(value)
        }
        Err(_) => {
            // Can't evaluate (might have free variables, or might error)
            // That's OK - just don't fold
            None
        }
    }
}
```

Example:

```melbi
// This map is built once at compile time
let blocklist = {
    "spam@example.com": true,
    "bad@actor.com": true,
    // ... hundreds more
}

// email.sender is impure (input variable)
// So this check runs at runtime
email.sender in blocklist
```

The optimizer recognizes `blocklist` is pure (no `~`) and constant-folds it,
while `email.sender` remains a runtime value.

### Migration Strategy

Effects are added incrementally:

**Phase 1** (Current): Type system foundation

- `EffectSet` and `Type` structures exist
- Effects are tracked but not enforced
- All expressions are `TOTAL` by default

**Phase 2**: Evaluator + Error Effect

- Implement evaluator/VM
- Identify all error sources
- Add `!` effect tracking to type checker
- Require `otherwise` for expressions with `!`
- Validate that evaluator errors match type checker's `!` tracking

**Phase 3**: Optimizer + Impure Effect

- Implement constant folding pass
- Add `~` effect tracking to type checker
- Mark input variables as `~`
- Constant fold expressions without `~`

**Phase 4**: Functions + Effect Propagation

- Implement lambda type checking
- Infer effects from lambda bodies
- Propagate effects through function calls
- Add FFI function effect declarations

No breaking changes at any phase - effects are additive.

### Work Required

#### Phase 2: Error Effect (After Evaluator)

- [ ] Add `!` tracking to type checker for all error sources
- [ ] Implement `otherwise` expression type checking
- [ ] Add top-level `!` check with helpful error messages
- [ ] Add tests for error propagation
- [ ] Validate evaluator errors match type checker tracking

Estimated: 3-5 days

#### Phase 3: Impure Effect (After Basic Optimizer)

- [ ] Mark input variables with `~` in type checker
- [ ] Add `~` tracking through all expressions
- [ ] Implement constant folding for expressions without `~`
- [ ] Add tests for constant folding
- [ ] Measure performance improvement

Estimated: 3-5 days

#### Phase 4: Function Effects

- [ ] Implement effect inference for lambda bodies
- [ ] Add function call effect propagation
- [ ] Implement `¬` (never) effect for short-circuiting
- [ ] Add FFI effect declarations
- [ ] Add tests for higher-order functions

Estimated: 5-7 days

### High-level Test Plan

**Unit tests** for each effect rule:

- Binary operations union effects correctly
- Division adds `!`
- Array indexing adds `!`
- `otherwise` removes `!`
- Input variables have `~`
- Constant folding only applies to expressions without `~`

**Integration tests**:

- Complex expressions with multiple effects
- Effect propagation through nested structures
- Error messages for unhandled `!`
- Constant folding optimization examples

**Property-based tests**:

- If expression has `~`, constant folding doesn't touch it
- If expression has `!`, top-level check catches it
- Effect union is commutative and associative

### Deployment Sequence

1. Merge Phase 1 (type system foundation) - already done
2. Implement and merge evaluator/VM
3. Merge Phase 2 (error effect) - enables "no crashes" guarantee
4. Implement and merge basic optimizer
5. Merge Phase 3 (impure effect) - enables constant folding
6. Merge Phase 4 (function effects) - completes effect system

## Impact

### Performance

**Compile-time**: Effect tracking adds minimal overhead to type checking (just
two booleans per type).

**Runtime**:

- Expressions with unhandled `!` are caught at compile time → zero runtime
  overhead
- Constant folding of pure expressions → significant performance gains
  - Example: Large map literals in filters don't need to be rebuilt on every
    evaluation

**Expected improvements**:

- 10-100x faster for expressions with large constant sub-expressions
- Zero runtime crashes (all errors handled at compile time)

### Security

Effects improve security by:

- Preventing unhandled errors that could leak information
- Making I/O operations explicit (visible via `~`)
- Enabling sandboxing policies based on effects (e.g., deny all `~` operations)

## Alternatives

### Alternative 1: Monadic Error Handling (like Rust's Result)

Have `Result<T, E>` type and force users to unwrap:

```melbi
let result = 10 / x  // Type: Result<Int, DivError>
let value = result?  // Unwrap with ?
```

**Pros**: Explicit, familiar to Rust developers

**Cons**:

- More verbose
- Allows `Vec<Result<Int>>` which breaks our "effects are on computations, not
  data" principle
- Users must think about error handling constantly

**Why we chose effects**: Automatic propagation, cleaner syntax, better
separation of concerns.

### Alternative 2: No Effect System

Just let things crash at runtime, or return error values.

**Pros**: Simpler implementation

**Cons**:

- Violates "no runtime errors" principle
- Can't do aggressive constant folding (don't know what's pure)
- Poor user experience (cryptic runtime errors)

**Why we chose effects**: Core to Melbi's value proposition.

### Alternative 3: Fine-grained Effect Tracking (per-parameter)

Track exactly which parameters contribute which effects:

```rust
pub struct FunctionEffects {
    param_0_contributes_error: bool,
    param_0_contributes_impure: bool,
    param_1_contributes_error: bool,
    param_1_contributes_impure: bool,
    // ...
}
```

**Pros**: More precise, better optimization potential

**Cons**: Significantly more complex, marginal benefit

**Why we chose conservative approach**: Start simple, add precision later if
needed.

## Looking into the Future

### Potential Extensions

**Multiple error types**:

```melbi
(10 / x) otherwise match {
    Ok(n) -> n,
    DivisionByZero -> 0,
    Overflow -> MAX_INT,
}
```

**Region effects** (like Rust lifetimes): Track borrowing and ownership if we
add mutable references.

**User-defined effects**: Allow FFI to define custom effects for domain-specific
tracking.

**Effect handlers**: Algebraic effect handlers (like Koka) for advanced control
flow.

**Permission-based effects**: Separate `~io` (I/O) from `~nd`
(non-deterministic) for finer-grained sandboxing policies.

### Next Steps

After basic effect system is working:

1. Add effect polymorphism visualization in error messages
2. Collect metrics on constant folding effectiveness
3. Consider exposing effects in LSP (show "this expression is pure" in hover)
4. Explore property-based testing for effect soundness
5. Write academic paper on "Practical Lightweight Effects for DSLs"

---

**Document Status**: Design phase - not yet implemented **Next Action**:
Implement evaluator, then add error effect tracking
