---
title: Error Handling System Design
---

# Design Doc: Error Handling, Collection, and Public API

**Author**: @NiltonVolpato

**Date**: 10-26-2025

## Introduction

### Background

Melbi aims to be an embeddable expression language with excellent error
messages. Research shows that compiler error quality significantly impacts
developer productivity
([amazingcto.com comparison](https://www.amazingcto.com/developer-productivity-compiler-errors/)).
Languages like Rust and Elm demonstrate that detailed, helpful error messages
with multiple code locations and suggestions dramatically improve the user
experience.

Current problems with error handling:

- **Internal errors were using miette**: Heavy std dependency, feature flag
  complexity, not suitable for no_std core
- **No error collection**: Stops at first error, forcing users to fix one issue
  at a time
- **No public API design**: Internal error types would leak to users, causing
  breaking changes
- **Inconsistent context**: No standard way to add provenance information to
  errors

**Stakeholders**:

- **Melbi users**: Need clear, actionable error messages
- **IDE/LSP developers**: Need structured diagnostics with spans
- **Embedding applications**: Need stable API that won't break
- **Melbi maintainers**: Need freedom to evolve error messages

### Current Functionality

Currently (as of this design):

- Errors are defined per-module with various structures
- miette is used for formatting (std-only, not compatible with no_std goal)
- Single error per operation (no collection)
- No clear boundary between internal and public error types

### In Scope

This design addresses:

1. **Internal error representation**: Rich, structured errors with full context
2. **Error collection mechanism**: Ability to return multiple errors from
   compilation
3. **Public API design**: Stable, opaque error types for library users
4. **Error categories**: Clear distinction between API, compilation, runtime,
   and resource errors
5. **LSP integration**: Structure that maps cleanly to LSP diagnostics
6. **no_std compatibility**: All error handling works without std

### Out of Scope

- Specific error messages for all ~20 error types (defined incrementally)
- Error recovery strategies during parsing
- Internationalization of error messages
- Error codes documentation website
- IDE quick-fix code actions (future LSP feature)

### Assumptions & Dependencies

- Arena allocation is used throughout Melbi (`bumpalo`)
- Errors can reference arena-allocated data (`&'a str`, `&'a Type<'a>`)
- Users want helpful messages more than terse output
- LSP will be built in the future and needs this structure

### Terminology

- **Diagnostic**: A single error, warning, or note with location
- **Span**: A range in source code (start/end offsets)
- **Context**: Related information about where/why an error occurred
- **Error Collection**: Gathering multiple errors in one compilation pass
- **Public API**: The stable interface exposed to library users
- **Internal Errors**: Rich error structures used within melbi-core

## Considerations

### Concerns

**Complexity vs Usability**:

- Rich error messages require complex internal structures
- Must balance detailed information with implementation simplicity
- Risk of over-engineering for ~20 error types

**Performance**:

- Error collection allocates Vecs during compilation
- String formatting for messages has overhead
- Arena allocation should mitigate most costs

**Breaking Changes**:

- Public API must be stable from v1.0
- Internal changes shouldn't affect users
- Need clear boundary between internal/external

### Operational Readiness Considerations

**Testing**:

- Error messages should have snapshot tests
- Multiple error collection needs comprehensive tests
- Public API needs stability tests (compile-fail tests)

**Debugging**:

- Internal errors should have Debug impl showing full detail
- Display impl should be user-friendly
- Error codes should link to documentation

**Evolution**:

- Error messages will improve over time
- New error types will be added
- Context types will expand

### Open Questions

1. Should error codes be required or optional? **Decision**: Optional, add as
   needed
2. How many errors should we collect before stopping? **Decision**: Collect all,
   let user decide
3. Should we support error code explanations like `rustc --explain`?
   **Decision**: Future enhancement
4. Should warnings be separate from errors? **Decision**: Use Severity enum

### Cross-Region Considerations

Not applicable - Melbi is a library, not a service.

## Proposed Design

### Solution

**Three-Layer Architecture**:

1. **Internal Layer**: Rich error structures with full type information

   - Lives in `core/src/error/` as private modules
   - Uses lifetimes to reference arena-allocated data
   - Tracks provenance via context chains
   - Multiple errors collected during compilation

2. **Conversion Layer**: Transforms internal → public

   - Converts rich types to strings/spans
   - Aggregates multiple internal errors
   - Categorizes errors appropriately

3. **Public API Layer**: Stable, opaque error enum
   - Four variants: Api, Compilation, Runtime, ResourceExceeded
   - No lifetimes, no internal types exposed
   - Can evolve internally without breaking users

### System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Public API (melbi)                       │
│                                                              │
│  pub enum Error {                                           │
│      Api(String),                                           │
│      Compilation { diagnostics: Vec<Diagnostic> },          │
│      Runtime { diagnostic: Diagnostic },                    │
│      ResourceExceeded { kind, limit, actual },              │
│  }                                                           │
│                                                              │
│  pub struct Diagnostic {                                    │
│      severity, message, span, related, help, code           │
│  }                                                           │
└─────────────────────────────────────────────────────────────┘
                              ▲
                              │ Conversion
                              │
┌─────────────────────────────────────────────────────────────┐
│              Internal Layer (melbi-core)                     │
│                                                              │
│  pub(crate) struct Error<'a> {                              │
│      kind: ErrorKind<'a>,     // Rich type info             │
│      source: &'a str,          // Arena allocated           │
│      context: Vec<Context<'a>>, // Reasoning chain          │
│      help: Option<&'a str>,    // Arena allocated           │
│  }                                                           │
│                                                              │
│  pub(crate) enum ErrorKind<'a> {                            │
│      TypeMismatch {                                         │
│          expected: &'a Type<'a>,  // Full type info         │
│          found: &'a Type<'a>,                               │
│          span: Span,                                        │
│      },                                                      │
│      UndefinedVariable { name: &'a str, span: Span },       │
│      // ... ~20 variants                                    │
│  }                                                           │
│                                                              │
│  pub(crate) enum Context<'a> {                              │
│      InFunctionCall { name: Option<&'a str>, span: Span },  │
│      WhileUnifying { what: &'a str, span: Span },           │
│      // ... ~10 variants                                    │
│  }                                                           │
│                                                              │
│  pub(crate) struct CheckResult<'a, T> {                     │
│      value: Option<T>,                                      │
│      errors: Vec<Error<'a>>,      // Multiple errors!       │
│      warnings: Vec<Warning<'a>>,                            │
│  }                                                           │
└─────────────────────────────────────────────────────────────┘
```

### Data Model

#### Internal Error Structure

```rust
// core/src/error/internal.rs
pub(crate) struct Error<'a> {
    pub kind: ErrorKind<'a>,
    pub source: &'a str,          // Arena-allocated source code
    pub context: Vec<Context<'a>>, // Provenance chain
    pub help: Option<&'a str>,    // Arena-allocated help text
}

pub(crate) enum ErrorKind<'a> {
    TypeMismatch {
        expected: &'a Type<'a>,
        found: &'a Type<'a>,
        span: Span,
    },
    ArgumentCount {
        expected: usize,
        found: usize,
        span: Span,
    },
    UndefinedVariable {
        name: &'a str,
        span: Span,
    },
    OccursCheck {
        var: &'a Type<'a>,
        in_type: &'a Type<'a>,
        span: Span,
    },
    // ... additional variants as needed
}

pub(crate) enum Context<'a> {
    InFunctionCall {
        name: Option<&'a str>,
        span: Span,
    },
    WhileUnifying {
        what: &'a str,  // "argument 0", "return type"
        span: Span,
    },
    DefinedHere {
        what: &'a str,  // "variable", "function"
        span: Span,
    },
    InferredHere {
        type_name: &'a str,
        span: Span,
    },
    // ... additional context types
}
```

#### Error Collection

```rust
// core/src/error/collector.rs
pub(crate) struct CheckResult<'a, T> {
    pub value: Option<T>,
    pub errors: Vec<Error<'a>>,
    pub warnings: Vec<Warning<'a>>,
}

pub(crate) struct ErrorCollector<'a> {
    errors: Vec<Error<'a>>,
    warnings: Vec<Warning<'a>>,
}

impl<'a> ErrorCollector<'a> {
    pub fn new() -> Self { /* ... */ }

    pub fn add(&mut self, err: Error<'a>) { /* ... */ }

    pub fn finish_with<T>(self, value: Option<T>) -> CheckResult<'a, T> {
        CheckResult {
            value,
            errors: self.errors,
            warnings: self.warnings,
        }
    }
}

// Extension trait for Result
pub(crate) trait OrCollect<'a, T> {
    fn or_collect(self, collector: &mut ErrorCollector<'a>) -> Option<T>;
}

impl<'a, T> OrCollect<'a, T> for Result<T, Error<'a>> {
    fn or_collect(self, collector: &mut ErrorCollector<'a>) -> Option<T> {
        match self {
            Ok(v) => Some(v),
            Err(e) => {
                collector.add(e);
                None
            }
        }
    }
}
```

#### Public API Types

```rust
// src/error.rs (public melbi crate)
#[derive(Debug, Clone)]
pub enum Error {
    /// API misuse (precondition/postcondition violations)
    Api(String),

    /// Compilation errors (syntax and semantic)
    Compilation {
        diagnostics: Vec<Diagnostic>,
    },

    /// Runtime error during evaluation
    Runtime {
        diagnostic: Diagnostic,
    },

    /// Resource limit exceeded
    ResourceExceeded {
        kind: ResourceKind,
        limit: usize,
        actual: Option<usize>,
    },
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    pub related: Vec<RelatedInfo>,
    pub help: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Memory,
    Time,
    StackDepth,
}
```

### Interface / API Definitions

#### Public API

```rust
// Main compilation function
pub fn compile(source: &str) -> Result<CompiledExpression, Error>

// Evaluation function
pub fn eval(expr: &CompiledExpression, ctx: &Context) -> Result<Value, Error>

// With resource limits
pub fn eval_with_config(
    expr: &CompiledExpression,
    ctx: &Context,
    config: &Config,
) -> Result<Value, Error>

// Error methods
impl Error {
    pub fn is_api_error(&self) -> bool
    pub fn is_compilation_error(&self) -> bool
    pub fn is_runtime_error(&self) -> bool
    pub fn is_resource_exceeded(&self) -> bool
    pub fn message(&self) -> String
    pub fn diagnostics(&self) -> &[Diagnostic]
}
```

#### Internal API

```rust
// Type checking returns multiple errors
pub(crate) fn type_check<'a>(
    expr: &'a Expr<'a>,
    arena: &'a Bump,
) -> CheckResult<'a, &'a Type<'a>>

// Error construction with context
let err = Error::new(
    ErrorKind::TypeMismatch { expected, found, span },
    source
);

// Add context to error
let err = err.with_context(Context::InFunctionCall {
    name: Some(func_name),
    span: call_span,
});

// Collect multiple errors
let mut collector = ErrorCollector::new();

let t1 = infer(left).or_collect(&mut collector);
let t2 = infer(right).or_collect(&mut collector);

collector.finish_with(Some(result_type))
```

### Business Logic

#### Error Collection Algorithm

During type checking:

1. Create `ErrorCollector`
2. For each sub-expression:
   - Attempt to check/infer type
   - If error, add to collector via `.or_collect()`
   - Continue checking remaining expressions
3. Return `CheckResult` with value (if possible) and all errors

**Example**:

```rust
pub fn check_function<'a>(
    func: &'a FunctionDef<'a>,
    arena: &'a Bump,
) -> CheckResult<'a, &'a Type<'a>> {
    let mut collector = ErrorCollector::new();

    // Check all parameters (collect all errors)
    let param_types: Vec<_> = func.params.iter()
        .filter_map(|p| check_param(p, arena).or_collect(&mut collector))
        .collect();

    // Check body even if params had errors
    let body_type = check_expr(func.body, arena).or_collect(&mut collector);

    // Check return type consistency
    if let Some(body_type) = body_type {
        unify(func.return_type, body_type, span)
            .or_collect(&mut collector);
    }

    collector.finish_with(Some(func_type))
}
```

#### Conversion to Public API

```rust
impl<'a> Error<'a> {
    pub(crate) fn to_diagnostic(&self) -> Diagnostic {
        // Convert ErrorKind to message string
        let (message, code) = match &self.kind {
            ErrorKind::TypeMismatch { expected, found, .. } => (
                format!("Type mismatch: expected {}, found {}", expected, found),
                Some("E001"),
            ),
            // ... other variants
        };

        // Get primary span from ErrorKind
        let span = self.kind.span();

        // Convert context to related info
        let related = self.context.iter()
            .map(|ctx| ctx.to_related_info())
            .collect();

        Diagnostic {
            severity: Severity::Error,
            message,
            span,
            related,
            help: self.help.map(|s| s.to_string()),
            code: code.map(|s| s.to_string()),
        }
    }
}

impl<'a, T> CheckResult<'a, T> {
    pub(crate) fn into_public_error(self) -> Result<T, Error> {
        if self.errors.is_empty() {
            Ok(self.value.expect("No errors but no value"))
        } else {
            let diagnostics = self.errors.iter()
                .map(|e| e.to_diagnostic())
                .chain(self.warnings.iter().map(|w| w.to_diagnostic()))
                .collect();

            Err(Error::Compilation { diagnostics })
        }
    }
}
```

### Migration Strategy

This is a new design, not a migration. Current ad-hoc error handling will be
replaced incrementally:

**Phase 1**: Implement internal error structures

- Define ErrorKind enum with initial variants
- Define Context enum
- Implement Error struct with context chain

**Phase 2**: Implement error collection

- Create CheckResult and ErrorCollector
- Refactor type checker to use CheckResult
- Add .or_collect() extension trait

**Phase 3**: Define public API

- Create public Error enum
- Implement conversion from internal → public
- Update melbi crate to use public API

**Phase 4**: Improve error messages

- Add help text to common errors
- Improve Display formatting
- Add error codes

### Work Required

1. **Core error structures** (~200 lines)

   - ErrorKind enum with ~5 initial variants
   - Context enum with ~5 initial variants
   - Error struct with context chain
   - Display implementations

2. **Error collection** (~150 lines)

   - CheckResult struct
   - ErrorCollector struct
   - OrCollect trait
   - Tests for multiple error collection

3. **Public API** (~150 lines)

   - Public Error enum
   - Diagnostic struct
   - Conversion functions
   - Error method implementations

4. **Type checker refactor** (~500 lines changes)

   - Switch from Result to CheckResult
   - Add error collection throughout
   - Test multiple errors returned

5. **Tests and documentation** (~300 lines)
   - Unit tests for error construction
   - Integration tests for multiple errors
   - Documentation examples

**Total estimate**: ~1300 lines of code + refactoring

### Work Sequence

1. Implement internal error structures (can be incremental)
2. Add error collection mechanism
3. Refactor one module (e.g., unification) to use CheckResult
4. Validate multiple errors work correctly
5. Define public API
6. Implement conversion layer
7. Refactor remaining modules
8. Update public melbi crate

### High-level Test Plan

**Unit Tests**:

- Error construction with context
- Context chain building
- ErrorCollector accumulation
- Conversion to public Diagnostic

**Integration Tests**:

```rust
#[test]
fn multiple_type_errors() {
    let source = r#"
        let x = 1 + "hello";
        let y = undefined_var;
        let z = x + y + unknown;
    "#;

    let result = compile(source);
    assert!(result.is_err());

    let Error::Compilation { diagnostics } = result.unwrap_err() else {
        panic!("Expected compilation error");
    };

    assert_eq!(diagnostics.len(), 3);
    assert!(diagnostics[0].message.contains("Type mismatch"));
    assert!(diagnostics[1].message.contains("Undefined"));
}
```

**Snapshot Tests**:

- Error message formatting
- Full diagnostic output
- Regression tests for error quality

### Deployment Sequence

Not applicable - this is a library design, not a deployment.

## Impact

### Performance

**Positive impacts**:

- Arena allocation means error strings are cheap
- Error collection allows fixing multiple issues per compile
- No heap fragmentation from individual error allocations

**Potential concerns**:

- Vec allocation for error collection (negligible)
- String formatting for messages (only on error path)
- Multiple passes to collect all errors (acceptable trade-off)

**Mitigation**:

- Benchmark type checking with/without collection
- Consider error collection limit if performance issues
- Profile memory usage of error Vecs

### Security

**Sandboxing**:

- ResourceExceeded errors enable safe sandboxing
- Time limits prevent infinite loops
- Memory limits prevent OOM attacks
- Stack depth limits prevent stack overflow

**Information Disclosure**:

- Error messages may reveal internal structure
- Spans could expose source code fragments
- Not a concern for Melbi's use case

### Usability

**Major positive impact**:

- Users see multiple errors at once
- Helpful context and suggestions
- Clear error categories for different handling
- LSP-ready structure for IDE integration

**Based on research**:

- Rust/Elm-style errors improve productivity
- Multiple locations in diagnostics help understanding
- Help text accelerates learning

### Cost Analysis

**Development cost**: ~1 week of implementation + 1 week refactoring

**Maintenance cost**: Low - stable public API means internal changes don't break
users

**Runtime cost**: Negligible - errors are on the cold path

## Alternatives

### Alternative 1: Use anyhow throughout

**Pros**: Very ergonomic, minimal code

**Cons**:

- Type erasure loses structured information
- Can't pattern match on error types
- Not suitable for library public API
- Loses LSP integration potential

**Rejected because**: Need structured errors for IDE support

### Alternative 2: Use snafu with feature flags

**Pros**: Derive macros reduce boilerplate, supports no_std

**Cons**:

- Feature flag complexity for error formatting
- Couples public API to derive macro behavior
- Harder to control exact error structure

**Rejected because**: Want full control over public API stability

### Alternative 3: Stop at first error (no collection)

**Pros**: Simpler implementation, less memory

**Cons**:

- Forces users to fix one error at a time
- Reduces productivity significantly
- Not competitive with modern compilers

**Rejected because**: Multiple error collection is essential for good UX

### Alternative 4: Expose internal error types publicly

**Pros**: No conversion overhead, users see full detail

**Cons**:

- Breaking changes every time we improve errors
- Can't evolve error messages without versioning
- Lifetime parameters leak into public API

**Rejected because**: Need stable public API for library users

## Looking into the Future

### Next Steps

1. **Error codes documentation**: Website explaining each error with examples
2. **Error code explanations**: `melbi --explain E001` like rustc
3. **LSP integration**: Map diagnostics directly to LSP protocol
4. **Quick fixes**: Suggest code actions in IDE
5. **Error recovery**: Continue parsing after syntax errors
6. **Internationalization**: Translate error messages

### Nice to Haves

- **Color output**: Colored error messages in terminal
- **Source code snippets**: Show code with highlighting in errors
- **Did you mean?**: Suggest similar names for typos
- **Type diff**: Show detailed type differences for complex types
- **Error analytics**: Track which errors are most common
- **Custom error renderers**: Let users customize error formatting

### Evolution Path

The design supports gradual enhancement:

- Error variants added internally without breaking public API
- Context types expanded as needed
- Display formatting improved continuously
- Error collection extended to parsing (currently just type checking)

The separation between internal and public errors means we can continuously
improve error quality without breaking users.
