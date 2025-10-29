# Design Doc: Melbi Public API

**Author**: @NiltonVolpato (with contributions from Claude)

**Date**: 10-21-2025

## Introduction

### Background

Melbi is a flexible, embeddable, expression-focused scripting language designed
for safe evaluation of user-defined logic in host applications. The primary use
cases include email filters, feature flags, data transformations, conditional
business rules, and any situation where end users need to define custom behavior
without modifying the host application's source code.

The public API is the primary interface between host applications (written in
Rust, C++, C, Python, JavaScript, etc.) and the Melbi runtime. It must support
both dynamic (runtime-checked) and static (compile-time checked) usage patterns,
provide excellent ergonomics for Rust users while remaining accessible from C
FFI, and enable safe sandboxing with predictable performance characteristics.

### Current Functionality

This is a new feature - the public API is being designed from the ground up.

### In Scope

- Context and expression compilation API
- Dynamic (runtime-checked) expression execution
- Static (compile-time checked) expression execution with type safety
- FFI system for registering host functions (both generic/polymorphic and
  monomorphic)
- Value construction and extraction APIs
- Type system access and management
- Error handling and reporting
- Arena-based memory management patterns

### Out of Scope

- Thread-safety and concurrent execution (deferred to future work)
- Serialization of compiled expressions
- Debugging and profiling APIs
- Advanced sandboxing controls (timeouts, memory limits)
- Package/module system details
- Async function support in FFI

### Assumptions & Dependencies

- Melbi uses arena allocation (bumpalo) for both types and runtime values
- The type system uses Hindley-Milner style inference with no annotations
  required
- Melbi supports parametric polymorphism (generics) with type erasure, similar
  to Java generics
- Host languages vary in capability - design must work for both C FFI and
  advanced type systems
- Users care about both safety (compile-time/runtime checks) and performance
  (zero-cost abstractions where possible)

### Terminology

- **Context**: The compilation environment containing the type manager and
  registered functions/packages
- **CompiledExpression**: A compiled Melbi expression ready for execution
- **TypedExpression**: A statically-typed wrapper around CompiledExpression
  providing compile-time safety
- **Value**: A runtime value in Melbi (tagged with its type)
- **Type**: A Melbi type (e.g., Int, String, Array[T], Map[K,V])
- **TypeManager**: Manages type allocation and interning in an arena
- **MelbiType**: Trait for Rust types that can be converted to/from Melbi types
- **MelbiArgs**: Trait for handling lists of arguments (implemented via Cons
  chains)
- **Cons**: Type-level cons list for representing heterogeneous argument lists
- **Arena**: Memory region for fast bump allocation (separate arenas for types
  vs runtime values)
- **FFI**: Foreign Function Interface - the mechanism for calling host language
  functions from Melbi

## Considerations

### Concerns

1. **Lifetime complexity**: Managing relationships between Context, Type arena,
   and runtime value arenas
2. **Thread safety**: Current design is single-threaded; multi-threading
   requires significant rework
3. **API surface area**: Three-tier design (unsafe/safe-dynamic/safe-static)
   creates more surface to maintain
4. **Cross-language consistency**: Ensuring similar ergonomics across Rust, C++,
   C, and dynamic languages
5. **Performance vs safety tradeoffs**: When to validate vs trust the type
   system
6. **Generic function representation**: How to express polymorphic Melbi
   functions in FFI

### Operational Readiness Considerations

Not applicable - this is a library, not a service.

### Open Questions

1. **Thread safety approach**: Arc-based sharing? bumpalo-herd? Clone contexts
   per thread?
2. **Serialization format**: If we add expression serialization, what format?
   (bytecode, JSON, binary?)
3. **Debugging hooks**: How should users debug Melbi expressions?
4. **Error spans**: How detailed should error location information be?
5. **Package system**: How are packages structured and loaded?
6. **Async FFI**: Should we support async Rust functions in FFI? How?

### Cross-Region Considerations

Not applicable - this is a library.

## Proposed Design

### Solution

The API is designed in three tiers:

1. **Unsafe/Unchecked API**: Maximum performance, no validation, assumes
   correctness
2. **Safe Dynamic API**: Runtime validation, works from C FFI and all host
   languages
3. **Safe Static API**: Compile-time type checking, ergonomic, Rust/C++ only

All tiers share the same core execution engine. The static API calls the unsafe
API after type conversions and compile-time validation, and the dynamic API
calls the unsafe API after runtime validation. This layering ensures consistency
while allowing users to choose their safety/performance tradeoff.

### System Architecture

```
┌─────────────────────────────────────────────┐
│         Host Application (User Code)        │
└──────────────────┬──────────────────────────┘
                   │
      ┌────────────┴────────────┐
      │                         │
┌─────▼──────┐         ┌───────▼─────┐
│   Static   │         │   Dynamic   │
│    API     │         │     API     │
│  (Rust)    │         │  (C FFI)    │
└─────┬──────┘         └──────┬──────┘
      │                       │
      │                ┌──────▼──────┐
      └───────────────>│   Unsafe    │
                       │     API     │
                       └──────┬──────┘
                              │
                    ┌─────────▼──────────┐
                    │  Melbi Core Engine │
                    │  - Type System     │
                    │  - Compiler        │
                    │  - VM/Interpreter  │
                    └────────────────────┘
```

### Data Model

#### Core Types

```rust
// Context owns the type arena and type manager
pub struct Context<'arena> {
    type_manager: TypeManager<'arena>,
}

// Compiled expression borrows from Context
pub struct CompiledExpression<'ctx, 'arena> {
    type_manager: &'ctx TypeManager<'arena>,
    source: String,
    param_types: Vec<&'arena Type<'arena>>,
    param_names: Vec<String>,
    return_type: &'arena Type<'arena>,
    bytecode: Vec<Op>, // or AST, depending on implementation
}

// Statically-typed wrapper
pub struct TypedExpression<'ctx, 'arena, Args, Ret> {
    inner: CompiledExpression<'ctx, 'arena>,
    _phantom: PhantomData<(Args, Ret)>,
}

// Type-level cons list for unlimited heterogeneous arguments
pub struct Cons<Head, Tail>(PhantomData<(Head, Tail)>);

// Runtime value
pub struct Value<'ty_arena, 'val_arena> {
    ty: &'ty_arena Type<'ty_arena>,
    data: ValueData<'val_arena>,
}
```

#### Lifetime relationships

- `'arena` (or `'ty_arena`): Lifetime of the type arena, tied to Context
- `'ctx`: Lifetime of borrowing the Context/TypeManager
- `'val` (or `'val_arena`): Lifetime of the runtime value arena, independent
  from type arena
- Constraint: `'arena: 'ctx` (type arena must outlive Context borrow)

### Interface / API Definitions

#### Dynamic Expression API

```rust
impl<'arena> Context<'arena> {
    // Create a new context with a type arena
    pub fn new(arena: &'arena Bump) -> Self;

    // Access the type manager
    pub fn type_manager(&self) -> &TypeManager<'arena>;

    // Compile an expression (dynamic API)
    pub fn compile<'ctx>(
        &'ctx self,
        source: &str,
        params: &[(&str, &'arena Type<'arena>)],
    ) -> Result<CompiledExpression<'ctx, 'arena>, CompileError>
    where
        'arena: 'ctx;
}

impl<'ctx, 'arena> CompiledExpression<'ctx, 'arena> {
    // Execute with runtime validation (safe dynamic API)
    pub fn run<'val>(
        &self,
        arena: &'val Bump,
        args: &[Value<'arena, 'val>],
    ) -> Result<Value<'arena, 'val>, RuntimeError>;

    // Execute without validation (unsafe API)
    pub fn run_unchecked<'val>(
        &self,
        arena: &'val Bump,
        args: &[Value<'arena, 'val>],
    ) -> Value<'arena, 'val>;

    // Metadata accessors
    pub fn param_types(&self) -> &[&'arena Type<'arena>];
    pub fn return_type(&self) -> &'arena Type<'arena>;
    pub fn param_names(&self) -> &[String];
    pub fn source(&self) -> &str;
}
```

#### Static Typing API

```rust
// Trait for types that can convert to/from Melbi
pub trait MelbiType: Sized {
    fn melbi_type<'arena>(ty_mgr: &TypeManager<'arena>) -> &'arena Type<'arena>;
    fn to_value<'arena, 'val>(
        self,
        arena: &'val Bump,
        ty: &'arena Type<'arena>,
    ) -> Value<'arena, 'val>;
    fn from_value<'arena, 'val>(
        val: Value<'arena, 'val>,
        ty_mgr: &TypeManager,
    ) -> Result<Self, RuntimeError>;
}

// Trait for argument lists (implemented recursively on Cons)
pub trait MelbiArgs {
    type Values;
    fn arg_types<'arena>(ty_mgr: &TypeManager<'arena>) -> Vec<&'arena Type<'arena>>;
    fn values_to_melbi<'arena, 'val>(
        values: Self::Values,
        arena: &'val Bump,
        types: &[&'arena Type<'arena>],
    ) -> Vec<Value<'arena, 'val>>;
}

// Compile with static types
impl<'arena> Context<'arena> {
    pub fn compile_typed<'ctx, Args, Ret>(
        &'ctx self,
        source: &str,
        param_names: &[&str],
    ) -> Result<TypedExpression<'ctx, 'arena, Args, Ret>, CompileError>
    where
        'arena: 'ctx,
        Args: MelbiArgs,
        Ret: MelbiType;
}

impl<'ctx, 'arena, Args, Ret> TypedExpression<'ctx, 'arena, Args, Ret>
where
    Args: MelbiArgs,
    Ret: MelbiType,
{
    // Type-safe execution (no runtime validation needed)
    pub fn eval<'val>(
        &self,
        arena: &'val Bump,
        args: Args::Values
    ) -> Result<Ret, RuntimeError>;
}
```

#### Macros for Ergonomics

```rust
// Macro to compile with function syntax
#[macro_export]
macro_rules! melbi_fn {
    ($ctx:expr, fn($($arg:ty),*) -> $ret:ty) => {
        |source: &str, param_names: &[&str]| {
            $ctx.compile_typed::<melbi_fn!(@cons_chain $($arg),*), $ret>(
                source,
                param_names
            )
        }
    };
    // ... helper rules to build Cons chain
}

// Macro to evaluate with flat argument syntax
#[macro_export]
macro_rules! melbi_eval {
    ($expr:expr, $arena:expr, $($arg:expr),*) => {
        $expr.eval($arena, melbi_eval!(@nest $($arg),*))
    };
    // ... helper rules to nest arguments into Cons structure
}
```

#### FFI Function Registration

Three-tier FFI design:

```rust
// 1. Unsafe/Raw FFI (C-compatible)
type RawMelbiFunction = fn(
    arena: &Bump,
    ty_mgr: &TypeManager,
    args: &[Value],
) -> Result<Value, RuntimeError>;

context.register_function_raw("add", raw_add_wrapper);

// 2. Dynamic Safe FFI (runtime validation, generic parameters)
context.register_generic_function(
    "get",
    &["K", "V"], // Type parameters
    &[
        TypeExpr::Map(box TypeExpr::Var("K"), box TypeExpr::Var("V")),
        TypeExpr::Var("K"),
        TypeExpr::Var("V"),
    ],
    TypeExpr::Var("V"),
    get_impl
);

// 3. Static FFI (Rust/C++ with compile-time checks)
#[melbi_function(K, V)]
fn get<K: MelbiType, V: MelbiType>(
    map: MelbiMap<K, V>,
    key: K,
    default: V,
) -> V {
    map.get(&key).unwrap_or(default)
}

register_fn![context, get];
```

### Business Logic

#### Cons-based Unlimited Arity

The key innovation is using type-level cons lists to represent heterogeneous
argument lists without arity limits:

```rust
// Type-level representation
Cons<i64, Cons<String, Cons<bool, ()>>>

// Value-level representation (nested tuples)
(42, ("hello", (true, ())))

// But users write (via macro):
melbi_eval![expr, &arena, 42, "hello", true]
```

The recursive `MelbiArgs` implementation handles any length:

```rust
impl<H: MelbiType, T: MelbiArgs> MelbiArgs for Cons<H, T> {
    type Values = (H, T::Values);
    // ... recursive conversion
}
```

#### Type Erasure for Generics

Melbi uses type erasure (like Java generics) rather than monomorphization:

- Generic functions operate on `Value` at runtime
- Type parameters are metadata for Melbi's type checker
- At FFI boundary, everything is `Value` with runtime type tags
- Enables polymorphic functions without code explosion

### Migration Strategy

Not applicable - this is a new API.

### Work Required

1. **Core API Implementation** (Rust)

   - Context and TypeManager integration
   - CompiledExpression structure
   - Dynamic run/run_unchecked methods
   - Error types and handling

2. **Static Typing Layer**

   - MelbiType trait and implementations (i64, String, bool, etc.)
   - MelbiArgs trait with Cons implementation
   - TypedExpression wrapper
   - melbi_fn! and melbi_eval! macros

3. **FFI System**

   - Raw function registration API
   - Dynamic generic function registration
   - Procedural macro for `#[melbi_function]`
   - Function signature extraction and validation

4. **C API Layer**

   - C-compatible function signatures
   - Manual memory management helpers
   - Error code translation

5. **Documentation**

   - API reference docs
   - Usage examples for each tier
   - Migration guide from dynamic to static API
   - FFI registration examples

6. **Testing**
   - Unit tests for each API tier
   - Integration tests showing interop
   - Performance benchmarks (validate zero-cost abstractions)
   - FFI function tests

### Work Sequence

1. Implement dynamic API first (Context, CompiledExpression, run/run_unchecked)
2. Add static typing layer (traits, Cons, TypedExpression)
3. Implement declarative macros (melbi_fn!, melbi_eval!)
4. Build FFI registration (raw → dynamic → static)
5. Develop procedural macro for `#[melbi_function]`
6. Create C API bindings
7. Documentation and examples

### High-level Test Plan

- **Unit tests**: Each trait implementation, macro expansion
- **Integration tests**: Full compile → execute workflows
- **Property tests**: Type safety guarantees, no panics in safe API
- **Performance tests**: Verify unchecked path is zero-overhead
- **FFI tests**: Calling Rust functions from Melbi expressions
- **Multi-language tests**: C API usage from other languages

### Deployment Sequence

Not applicable - this is a library.

## Impact

### Performance

- **Unsafe API**: Zero overhead, direct execution
- **Dynamic API**: Single validation pass, negligible overhead
- **Static API**: Zero runtime overhead after type conversion, calls unsafe path
- **Arena allocation**: Fast bump allocation, batch deallocation
- **Type erasure**: No monomorphization code bloat for generics

### Security

- Type arena and value arena separation prevents type confusion
- Sandboxing to be implemented (timeouts, memory limits) - deferred
- Safe dynamic API prevents invalid type access
- Static API provides compile-time guarantees

### Other Aspects

- **Memory**: Arena-based allocation is cache-friendly and predictable
- **API Surface**: Large but consistent - three tiers cover all use cases
- **Maintainability**: Layered design with clear separation of concerns
- **Extensibility**: New types/functions can be added via traits

### Cost Analysis

Not applicable - this is a library, not a service.

### Cross-Region Considerations

Not applicable - this is a library.

## Alternatives

### Alternative 1: Single Dynamic API Only

**Description**: Provide only the dynamic (C-compatible) API without static
typing.

**Why discarded**:

- Misses opportunity for compile-time safety in Rust/C++
- Less ergonomic for typed host languages
- Loses performance benefits of skipping validation

### Alternative 2: Tuple-based Arguments with Arity Limit

**Description**: Use regular Rust tuples for arguments, accept 12-argument
limit.

**Why discarded**:

- Arbitrary limitation frustrates users
- Cons-based approach provides unlimited arity without complexity
- Would still need fallback to dynamic API for >12 args

### Alternative 3: Macro-only Static API

**Description**: Use only declarative macros, no trait system.

**Why discarded**:

- Macros can't express recursive structures cleanly
- Less composable than trait-based approach
- Harder to extend by users

### Alternative 4: Monomorphization Instead of Type Erasure

**Description**: Generate specialized code for each type combination (like
Rust/C++).

**Why discarded**:

- Code bloat for generic functions
- Compilation complexity
- Type erasure is simpler and sufficient for Melbi's use cases

## Looking into the Future

### Next Steps

1. **Thread Safety**: Design Arc-based or bumpalo-herd approach for
   multi-threading
2. **Serialization**: Add bytecode serialization for caching compiled
   expressions
3. **Debugging API**: Breakpoints, step execution, variable inspection
4. **Advanced Sandboxing**: Implement timeout, memory limit, and operation
   allowlist
5. **Async FFI**: Support async Rust functions in FFI layer
6. **Package System**: Design module/package loading and namespacing
7. **Language Bindings**: Python, JavaScript, Java, Go bindings on top of C API

### Nice to Haves

- REPL for interactive Melbi development
- VS Code extension with LSP support
- Performance profiler for Melbi expressions
- Hot reloading of expressions
- Expression optimizer (constant folding, dead code elimination)
- JIT compilation for hot paths

---

**Document Status**: Initial design **Last Updated**: October 21, 2025
**Maintainers**: @NiltonVolpato
