---
title: Melbi Formal Verification with CBMC
---

# Design Doc: Formal Verification Integration for Melbi

**Author**: @NiltonVolpato

**Date**: 10-29-2025

**Status**: Draft - Starting point for detailed design

## Introduction

### Background

Melbi is an embeddable expression language with strong safety guarantees through its type system and effect tracking. However, type systems can only prevent certain classes of errors - they cannot prove that complex logic is correct for all possible inputs.

**The Opportunity**: By integrating formal verification tools like CBMC (C Bounded Model Checker), Melbi can provide **mathematical proofs of correctness** that go beyond what types and tests can offer. This would make Melbi suitable for safety-critical applications in automotive, aerospace, medical devices, and financial systems.

**The Vision**: Create a three-tier safety model where each tier provides increasing levels of assurance:
- **Tier 1: Type Safety** (always on) - Prevents type errors, tracks effects
- **Tier 2: Example-Based Testing** (opt-in) - Validates specific test cases
- **Tier 3: Formal Verification** (opt-in) - Proves properties hold for ALL inputs

**Stakeholders**:
- Safety-critical system developers (automotive, aerospace, medical)
- Financial system developers requiring auditable correctness
- Smart contract developers needing provable properties
- Regulatory compliance teams needing certification evidence

### Current Functionality

As of this design document, Melbi has:
- Type system with effect tracking (`!` for errors, `~` for impure)
- Expression-only design (no statements)
- Built-in test system (see `docs/design/unit-testing.md`)
- Metadata directive system (see `docs/design/metadata-directives.md`)
- No formal verification capabilities

### In Scope

This design addresses:
- Integration with CBMC for bounded model checking
- Translation from Melbi expressions to C code for verification
- Verification directives: `%verify`, `%property`, `%bounded`
- API for running verification and reporting results
- Counter-example generation for failed properties
- CI/CD integration workflow

### Out of Scope

The following are explicitly NOT included in this initial design:

- Alternative verification backends (Z3, Dafny, Coq) - future extensions
- Unbounded verification (CBMC is bounded by design)
- Interactive theorem proving
- Proof certificates or proof objects
- Verification of host-provided functions (FFI)
- Verification across multiple Melbi expressions
- Performance optimization of verification time

### Assumptions & Dependencies

- CBMC is installed and available in CI/CD environment
- Melbi expressions can be translated to equivalent C code
- Effect system (`~` impure) accurately identifies non-deterministic operations
- Arena allocation provides bounded memory model
- Host functions are assumed correct (trusted)
- Verification runs offline in CI/CD, not at runtime

### Terminology

- **Formal Verification**: Mathematical proof that code satisfies specified properties
- **CBMC**: C Bounded Model Checker - tool for verifying C programs
- **Model Checking**: Automated technique for verifying finite-state systems
- **Property**: A mathematical assertion that should hold for all inputs
- **Counter-example**: Specific input values that violate a property
- **Bounded Verification**: Verification up to a specific depth/bound
- **SMT Solver**: Satisfiability Modulo Theories solver (backend for CBMC)

## Considerations

### Concerns

**Translation Complexity**: Translating Melbi's high-level constructs (lambdas, closures, effects) to C is non-trivial. Need to ensure translation preserves semantics accurately.

**Verification Time**: CBMC can be slow for complex expressions. Need timeouts and bounds to prevent CI/CD blocking.

**User Experience**: Verification errors can be cryptic. Need clear error messages that map back to Melbi source, not generated C code.

**False Positives**: Bounded verification might miss edge cases beyond bounds. Need clear communication about limitations.

**Host Function Trust**: Verification assumes host-provided functions are correct. Bugs in host functions won't be caught.

### Operational Readiness Considerations

**CI/CD Integration**: Verification runs as optional CI/CD step. Only expressions with `%verify` directives trigger verification.

**Performance**: Verification can take seconds to minutes. Need:
- Configurable timeouts (default 30 seconds)
- Parallel verification for multiple expressions
- Caching of verification results

**Error Reporting**: When verification fails:
- Show counter-example in Melbi syntax, not C
- Highlight which property failed
- Suggest fixes (e.g., add bounds checks, add `otherwise`)

**Resource Usage**: CBMC can be memory-intensive. Monitor and limit memory usage in CI/CD.

### Open Questions

1. **How to handle higher-order functions?** CBMC works with C, which doesn't have first-class functions. Do we:
   - Inline all lambdas (limits expressiveness)?
   - Use function pointers (limits verification)?
   - Restrict verification to non-higher-order expressions?

2. **What verification bounds are reasonable defaults?**
   - Recursion depth: 10? 100?
   - Loop unrolling: 1000 iterations?
   - Array sizes: 1000 elements?

3. **How to verify impure operations?** Operations marked `~` depend on runtime context. Do we:
   - Reject verification of impure expressions?
   - Model them as non-deterministic input?
   - Require `%disallow impure` for verification?

4. **Should verification be incremental?** Can we cache verification results and only re-verify when expressions change?

5. **How to handle verification of recursive functions?** Need termination proofs or bounded unrolling.

6. **Property syntax: Melbi-like or separate language?** Should properties use Melbi syntax or a separate specification language?

### Cross-Region Considerations

Not applicable - verification runs in CI/CD environments, not production across regions.

## Proposed Design

### Solution

Integrate CBMC as an optional verification backend for Melbi expressions. Users opt in via metadata directives (`%verify`, `%property`, `%bounded`). During CI/CD, expressions with verification directives are:

1. Translated from Melbi to C code
2. Augmented with CBMC assertions for properties
3. Verified by CBMC with specified bounds
4. Results reported back in Melbi syntax

**Key Design Principles**:
- **Opt-in**: Verification is optional, doesn't impact non-verified code
- **Bounded**: Use CBMC's bounded model checking (practical for CI/CD)
- **Composable**: Works alongside type checking and testing
- **Clear errors**: Counter-examples shown in Melbi syntax, not C

### System Architecture

```
┌──────────────────────────────────────────────┐
│  Melbi Source with Verification Directives   │
│  %verify no-errors, deterministic            │
│  %property "forall x: x >= 0 => result >= 0" │
└────────────────┬─────────────────────────────┘
                 │
                 ▼
        ┌────────────────┐
        │  Parser        │
        │  - Directives  │
        │  - Expression  │
        │  - Properties  │
        └────────┬───────┘
                 │
                 ▼
        ┌────────────────┐
        │  Type Checker  │
        │  - Validate    │
        │  - Effects     │
        └────────┬───────┘
                 │
                 ▼
        ┌──────────────────────┐
        │  Verification Engine │
        │  (if %verify present)│
        └────────┬─────────────┘
                 │
                 ▼
        ┌──────────────────────┐
        │  Melbi → C Translator│
        │  - AST to C          │
        │  - Add assertions    │
        │  - Handle effects    │
        └────────┬─────────────┘
                 │
                 ▼
        ┌──────────────────────┐
        │  Generated C Code    │
        │  with CBMC markers   │
        └────────┬─────────────┘
                 │
                 ▼
        ┌──────────────────────┐
        │  CBMC                │
        │  - Model check       │
        │  - Generate proof    │
        │  - Or counter-example│
        └────────┬─────────────┘
                 │
                 ▼
        ┌──────────────────────┐
        │  Result Translator   │
        │  - C → Melbi syntax  │
        │  - Format errors     │
        └────────┬─────────────┘
                 │
                 ▼
        ┌──────────────────────┐
        │  Verification Report │
        │  - Pass/Fail         │
        │  - Counter-examples  │
        └──────────────────────┘
```

### Data Model

#### Verification Directives (from metadata-directives.md)

```rust
#[derive(Debug, Clone)]
pub enum Directive {
    // ... existing directives ...

    Verify(Vec<VerifyCheck>),        // %verify no-errors, bounds
    Property(String),                 // %property "forall x: ..."
    Bounded { depth: u32, unroll: u32 }, // %bounded depth: 10, unroll: 100
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyCheck {
    NoErrors,      // Prove no runtime errors
    Deterministic, // Prove always same output for same input
    Bounds,        // Prove values stay within bounds
    Overflow,      // Prove no integer overflow
    Termination,   // Prove recursion terminates
}
```

#### Verification Configuration

```rust
#[derive(Debug, Clone)]
pub struct VerificationConfig {
    pub enabled: bool,
    pub checks: HashSet<VerifyCheck>,
    pub properties: Vec<Property>,
    pub bounds: VerificationBounds,
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct VerificationBounds {
    pub max_recursion_depth: u32,
    pub max_loop_unroll: u32,
    pub max_array_size: u32,
}

impl Default for VerificationBounds {
    fn default() -> Self {
        Self {
            max_recursion_depth: 10,
            max_loop_unroll: 1000,
            max_array_size: 1000,
        }
    }
}
```

#### Property Representation

```rust
/// A property to verify
#[derive(Debug, Clone)]
pub struct Property {
    pub source: String,      // Original property string
    pub kind: PropertyKind,
}

#[derive(Debug, Clone)]
pub enum PropertyKind {
    Forall {
        vars: Vec<(String, Type)>,  // Quantified variables
        condition: Expr,             // Must hold for all values
    },
    Exists {
        vars: Vec<(String, Type)>,
        condition: Expr,
    },
    Invariant(Expr),  // Must hold throughout execution
}
```

#### Verification Results

```rust
#[derive(Debug)]
pub struct VerificationResult {
    pub success: bool,
    pub checks_passed: Vec<VerifyCheck>,
    pub checks_failed: Vec<CheckFailure>,
    pub properties_verified: Vec<String>,
    pub properties_violated: Vec<PropertyViolation>,
    pub verification_time: Duration,
}

#[derive(Debug)]
pub struct CheckFailure {
    pub check: VerifyCheck,
    pub counter_example: Option<CounterExample>,
    pub error_message: String,
}

#[derive(Debug)]
pub struct PropertyViolation {
    pub property: String,
    pub counter_example: CounterExample,
}

#[derive(Debug)]
pub struct CounterExample {
    pub inputs: HashMap<String, Value>,
    pub trace: Vec<String>,  // Execution trace leading to violation
}
```

### Interface / API Definitions

#### Verification API

```rust
/// Main verification entry point
pub fn verify_expression(
    expr: &Expr,
    config: &VerificationConfig,
    context_schema: &ContextSchema,
) -> Result<VerificationResult, VerificationError>;

/// Translate Melbi expression to C code for CBMC
pub fn translate_to_c(
    expr: &Expr,
    properties: &[Property],
    bounds: &VerificationBounds,
) -> Result<String, TranslationError>;

/// Run CBMC on generated C code
pub fn run_cbmc(
    c_code: &str,
    timeout: Duration,
) -> Result<CbmcOutput, CbmcError>;

/// Translate CBMC counter-example back to Melbi
pub fn translate_counter_example(
    cbmc_output: &CbmcOutput,
    expr: &Expr,
) -> CounterExample;
```

#### Host Integration API

```rust
impl HostConfig {
    /// Enable verification with custom configuration
    pub fn enable_verification(&mut self, config: VerificationConfig);

    /// Set verification timeout
    pub fn set_verification_timeout(&mut self, timeout: Duration);

    /// Add trusted host functions (not verified)
    pub fn add_trusted_function(&mut self, name: &str);
}

impl Engine {
    /// Verify an expression (used in CI/CD)
    pub fn verify(&self, source: &str) -> Result<VerificationResult, Error>;

    /// Check if expression has verification directives
    pub fn has_verification(&self, source: &str) -> bool;
}
```

### Business Logic

#### Verification Checks

**No-Errors Check**:
- Proves no division by zero
- Proves array accesses are in bounds
- Proves no integer overflow/underflow
- Proves all `otherwise` fallbacks are unnecessary (or necessary)

**Deterministic Check**:
- Identifies all impure operations (marked with `~`)
- Proves expression depends only on inputs
- Fails if any impure operations present

**Bounds Check**:
- For numeric results, proves values stay within specified ranges
- Useful for physical quantities (speed >= 0, percentage in 0-100)

**Overflow Check**:
- Proves no integer arithmetic overflows
- Particularly important for safety-critical calculations

**Termination Check**:
- Proves all recursive calls eventually terminate
- Requires either: induction proof or bounded unrolling

#### Translation Strategy

**Basic Types**:
```
Melbi Type    →  C Type
─────────────────────────
Int           →  int64_t
Float         →  double
Bool          →  bool
String        →  char* (bounded)
Array[T]      →  T* (bounded)
Record        →  struct
```

**Operations**:
```c
// Melbi: x / y
// C translation:
__CPROVER_assert(y != 0, "Division by zero");
result = x / y;

// Melbi: arr[i]
// C translation:
__CPROVER_assert(i >= 0 && i < arr_len, "Array index out of bounds");
result = arr[i];
```

**Properties**:
```
// Melbi: %property "forall x: x >= 0 => result >= 0"
// C translation:
int x = nondet_int();
__CPROVER_assume(x >= 0);
int result = evaluate_expression(x);
__CPROVER_assert(result >= 0, "Property violated");
```

#### Handling Effects

**Pure expressions (`!` and no `~`)**:
- Can be fully verified
- All behavior is deterministic given inputs

**Expressions with `!` (errors)**:
- Verification proves errors are handled (via `otherwise`)
- Or proves errors never occur

**Expressions with `~` (impure)**:
- Cannot verify determinism
- Model impure operations as non-deterministic input
- Or require `%disallow impure` for full verification

### Migration Strategy

No migration needed - this is a new, opt-in feature.

**Adoption Path**:
1. Users can add `%verify` to new expressions
2. CI/CD can be configured to require verification for specific directories
3. Existing expressions continue working without verification

**Gradual Rollout**:
- Phase 1: Simple expressions (no recursion, no higher-order functions)
- Phase 2: Recursive expressions with bounded depth
- Phase 3: Higher-order functions (if feasible)

### Work Required

#### Phase 1: Foundation (3-5 weeks)
- Add verification directives to parser
- Design verification API and data structures
- Implement trivial verification (constant expressions)
- Manual checks without CBMC (proof of concept)
- Testing and documentation

#### Phase 2: CBMC Integration (12-16 weeks)
- Implement Melbi → C translator for basic types
- Handle arithmetic operations with assertions
- Integrate CBMC toolchain
- Counter-example translation
- Property parsing and translation
- Error message formatting

#### Phase 3: Advanced Features (8-12 weeks)
- Recursive function verification
- Array/collection operations
- Record/struct handling
- Complex property expressions
- Optimization and caching

#### Phase 4: Production Readiness (4-6 weeks)
- CI/CD integration examples
- Performance optimization
- Comprehensive testing
- User documentation
- Example gallery

**Total Estimate**: 27-39 weeks (6-9 months) for full implementation

**External Dependencies**:
- CBMC installation in CI/CD
- Understanding of CBMC's C dialect and limitations

### Work Sequence

1. **Foundation** - Must complete before CBMC integration
2. **Basic CBMC integration** - Can develop incrementally
3. **Advanced features** - Can be prioritized based on user needs
4. **Production hardening** - Final polish

### High-level Test Plan

#### Unit Tests
- Translation of each Melbi construct to C
- Property parsing and translation
- Counter-example translation
- Bounds validation

#### Integration Tests
- End-to-end: Melbi → C → CBMC → Result
- Simple properties (x >= 0)
- Complex properties (forall/exists)
- Error cases (violated properties)

#### Real-World Examples
- Division by zero prevention
- Array bounds checking
- Financial calculations
- State machine verification

#### Performance Tests
- Verification time for various expression sizes
- Timeout handling
- Memory usage

### Deployment Sequence

1. Merge Phase 1 (directives and API design)
2. Merge Phase 2 (basic CBMC integration)
3. Beta release with documentation
4. Gather feedback from early adopters
5. Implement Phase 3 based on feedback
6. Production release with examples

## Impact

### Performance Impact

**Compilation**: No impact - verification is optional and offline.

**CI/CD**: Can add minutes to pipeline for verified expressions. Mitigation:
- Only verify expressions with `%verify` directive
- Run verification in parallel
- Cache verification results

**Runtime**: Zero impact - verification happens offline, not in production.

### Security Impact

**Positive**:
- Proves absence of certain vulnerabilities (overflow, bounds errors)
- Increases confidence in safety-critical code
- Provides audit trail for compliance

**Negative**:
- None - verification is opt-in and doesn't affect runtime

### Developer Experience Impact

**Positive**:
- High confidence in correctness
- Early detection of edge cases
- Better understanding of code behavior
- Compliance evidence for certification

**Negative**:
- Learning curve for property writing
- Potentially slow feedback (minutes vs seconds)
- Counter-examples can be hard to interpret

**Mitigation**:
- Excellent documentation with examples
- Clear error messages mapping to Melbi source
- Progressive disclosure (simple checks first)

### Cost Analysis

**Development**: 6-9 months of focused engineering effort.

**CI/CD**: Additional compute for verification (minutes per expression).

**Maintenance**: Ongoing updates as CBMC evolves, new properties added.

### Cross-Region Considerations

Not applicable - verification is a development/CI tool, not deployed to production.

## Alternatives

### Alternative 1: Z3 SMT Solver

Use Z3 instead of CBMC for verification.

**Pros**:
- More powerful theorem proving
- Better handling of quantifiers
- Mature Rust bindings

**Cons**:
- Different translation strategy needed
- Less familiar to C developers
- Potentially harder to debug

**Verdict**: Consider for future, but start with CBMC (better known in safety-critical domains).

### Alternative 2: Dafny Integration

Translate Melbi to Dafny for verification.

**Pros**:
- Very powerful verification
- First-class support for specifications
- Active development

**Cons**:
- Another language to learn
- Translation complexity
- Overkill for embedded expressions

**Verdict**: Too heavyweight for initial implementation.

### Alternative 3: Proof-Carrying Code

Generate proof certificates that travel with code.

**Pros**:
- Verification once, trust everywhere
- No need to re-verify

**Cons**:
- Complex implementation
- Large proof objects
- Limited tool support

**Verdict**: Interesting for future, but not practical now.

### Alternative 4: Property-Based Testing

Use property-based testing (like QuickCheck) instead of formal verification.

**Pros**:
- Easier to implement
- Faster feedback
- Good for finding bugs

**Cons**:
- Not exhaustive (can't prove correctness)
- Not suitable for safety-critical certification
- Already covered by test system

**Verdict**: Complementary, not alternative. Both are valuable.

## Looking into the Future

### Immediate Next Steps (Post-Phase 1)

- Gather user feedback on property syntax
- Identify most common verification use cases
- Build example gallery
- Optimize translation performance

### Medium-Term Enhancements

**Alternative Backends**:
- Z3 for theorem proving
- Symbolic execution engines
- Abstract interpretation

**Richer Properties**:
- Temporal logic (LTL, CTL)
- Relational properties
- Information flow properties

**Better Diagnostics**:
- Visualization of counter-examples
- Minimization of counter-examples
- Suggested fixes

### Long-Term Vision

**Verification-Driven Development**:
- IDE integration with inline verification
- Real-time verification feedback
- Automatic property generation from types

**Certified Compilation**:
- Prove compiler correctness
- End-to-end correctness from source to execution
- Certification evidence generation

**Community Library**:
- Shared verified expressions
- Reusable properties and patterns
- Benchmarks and case studies

---

## Appendix: Examples

### Example 1: Division Safety

```melbi
%melbi 2
%verify no-errors
%property "forall x, y: y != 0 => result == x / y"
---
(numerator / denominator) otherwise 0 where {
    denominator = if divisor == 0 then 1 else divisor
}
```

**What CBMC Proves**:
1. No division by zero can occur
2. When divisor ≠ 0, result is correct quotient
3. When divisor = 0, fallback value (0) is returned

### Example 2: Bounds Checking

```melbi
%melbi 2
%verify no-errors, bounds
%property "forall speed: result >= 0"
%property "forall speed: speed > 100 => result > 100"
---
if speed < 0 then 0
else if speed > 200 then 200
else speed
```

**What CBMC Proves**:
1. Result is never negative
2. Result respects clamping behavior
3. High speeds remain high after clamping

### Example 3: Array Safety

```melbi
%melbi 2
%verify no-errors
%bounded unroll: 100
---
sum([x * x for x in numbers])
```

**What CBMC Proves**:
1. No array out-of-bounds access
2. No overflow in accumulation (with bounds)
3. Correct summation logic

### Example 4: Financial Calculation

```melbi
%melbi 2
%doc "Calculate compound interest - must be deterministic for auditing"
%disallow impure
%verify deterministic, no-errors, overflow
%property "forall principal, rate, years: result >= principal"
---
principal * (1 + rate) ^ years
```

**What CBMC Proves**:
1. Deterministic (no random/time-dependent operations)
2. No overflow in exponentiation
3. Result is never less than principal (for positive rate)

---

**Document Status**: Draft - Starting point for full design
**Last Updated**: October 29, 2025
**Next Steps**: Detailed design session, refine open questions, prototype translation
