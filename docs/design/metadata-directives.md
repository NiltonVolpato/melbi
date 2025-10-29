---
title: Melbi Metadata Directives - Technical Design Document
---

# Design Doc: Melbi Metadata Directives

**Author**: @NiltonVolpato

**Date**: 10-29-2025

## Introduction

### Background

Melbi is an embeddable, expression-focused language designed for safe evaluation of user code. As the language evolves and is deployed in production environments, we need mechanisms to:

1. **Version expressions** - Allow breaking changes while maintaining backward compatibility
2. **Document intent** - Enable self-documenting expressions for team collaboration
3. **Control safety properties** - Let users opt into stricter guarantees (determinism, error handling)
4. **Enable experimental features** - Support gradual rollout of new language features

Currently, Melbi expressions are pure code with no metadata capabilities. This makes it difficult to evolve the language, enforce team standards, or provide context for future maintainers.

The solution is a **metadata directive system** using a YAML-inspired syntax that separates configuration from code while maintaining Melbi's expression-focused philosophy.

### Current Functionality

As of this design document, Melbi supports:
- Pure expression syntax with no metadata
- No versioning mechanism
- No built-in documentation support
- No user-configurable strictness controls
- No experimental feature flags

All configuration must be done at the host/runtime level with no visibility to the expression author.

### In Scope

This design covers:
- Syntax for metadata directives (using `%` prefix)
- Core directives: `%melbi`, `%doc`, `%allow`, `%disallow`, `%experimental`
- Three-section structure: directives, expression, tests (separated by `---`)
- Strictness enforcement model (users can only go stricter)
- Host control mechanisms (defaults, freezing, relaxation)
- Parser modifications to support directives
- Integration with type checker for effect enforcement

### Out of Scope

The following are explicitly NOT included in this design:

- Package/module declarations (`%use Stats`) - packages are records, no special syntax needed
- Optimization hints (`%optimize aggressive`) - deferred to future
- Type coercion flags (`%allow coercion`) - conflicts with explicit-is-better philosophy
- User-defined resource limits (`%timeout`, `%memory`) - security concern, host-controlled only
- Multiple error types - current design assumes single error effect
- Directive inheritance or includes

### Assumptions & Dependencies

- Parser is based on Pest grammar and can be extended with new rules
- Effect system (`!` for errors, `~` for impure) is implemented or in progress
- Type checker can enforce `disallow` constraints during compilation
- Host applications have a configuration API to set defaults and freeze values
- Three-section structure (directives/expression/tests) is acceptable

### Terminology

- **Directive**: Metadata instruction prefixed with `%` that configures language behavior
- **Effect**: Type-level property (`!` error, `~` impure) tracked by the compiler
- **Strictness**: A partial ordering where `disallow` is always stricter than `allow`
- **Freezing**: Host mechanism to prevent users from changing a directive value
- **Baseline**: Default directive values set by the host environment
- **Relaxation**: Host-permitted change from stricter to more permissive (rare)

## Considerations

### Concerns

**Complexity**: Adding metadata could make simple expressions feel heavyweight. Mitigation: all directives are optional, simple expressions need no directives.

**Parsing ambiguity**: Need to clearly distinguish directives from expression code. Mitigation: `%` prefix is unambiguous, never used in expression syntax.

**Version proliferation**: If every expression declares `%melbi 2`, it becomes noise. Mitigation: directives are optional; only needed when defaults aren't sufficient or when documentation is valuable.

**Learning curve**: New users need to understand when directives matter. Mitigation: excellent error messages that suggest appropriate directives when issues arise.

**Host complexity**: Hosts need to manage defaults, freezing, and validation. Mitigation: simple API with sensible defaults, advanced features are opt-in.

### Operational Readiness Considerations

**Deployment**: This is a language-level feature requiring:
- Parser updates (Pest grammar modifications)
- Type checker integration for effect enforcement
- Host API additions for configuration
- Documentation and examples

**Monitoring**: Track usage patterns:
- Which directives are most commonly used
- How often users override defaults
- Frequency of strictness violations (rejected expressions)

**Debugging**: When expressions fail due to directive mismatches:
- Clear error messages showing expected vs actual
- Suggestions for resolution
- Host configuration visibility

**Migration**: Existing expressions without directives continue to work (all directives optional). New features can be gated behind `%experimental` for smooth rollout.

### Open Questions

1. **Should `%melbi <version>` be required immediately or only after first breaking change?**
   - Current plan: Optional initially, required after we introduce breaking changes post-launch
   - Allows gradual adoption while we stabilize

2. **How do we handle directive conflicts?** (e.g., `%allow errors` followed by `%disallow errors`)
   - Proposal: Last directive wins, with warning
   - Alternative: Treat as error
   - Decision needed before implementation

3. **Should directives be allowed after the expression starts?**
   - Proposal: No, all directives must come before `---` or first non-directive line
   - Keeps parsing simple and structure clear

4. **What happens if host default is `disallow X` but doesn't freeze it?**
   - User writes `%allow X` - should this error or be a no-op?
   - Proposal: Error by default (can't relax), unless host calls `allow_relaxation("X")`

5. **Should the test section be part of this design or separate?**
   - Tests are orthogonal to directives
   - Could be separate design doc, but synergy with three-section structure
   - Proposal: Acknowledge test section structure, detail test syntax in separate doc
   - **See**: `docs/design/unit-testing.md` for full test system specification

6. **How do verification directives interact with allow/disallow?**
   - If `%verify no-errors` is specified, should we implicitly `%disallow errors`?
   - Or are they independent (verification proves it, disallow enforces it)?
   - Proposal: Keep independent - verification is proof, allow/disallow is policy
   - This allows: "verify it's safe, but still require `otherwise` for clarity"

### Cross-Region Considerations

Not applicable - Melbi is a language specification, not a deployed service. Implementation considerations are per-host.

## Proposed Design

### Solution

Introduce a metadata directive system using `%` prefixed keywords at the start of a Melbi expression. Directives are separated from the expression code by `---` (optional but recommended for clarity). The system supports:

1. Language versioning (`%melbi`)
2. Self-documentation (`%doc`)
3. Strictness controls (`%allow`, `%disallow`)
4. Experimental features (`%experimental`)

Directives are processed before type checking, allowing them to influence compiler behavior. The host can set baseline defaults and freeze values to enforce security/correctness policies.

### System Architecture

```
┌─────────────────────────────────────────────┐
│  Melbi Source Text                          │
│  %melbi 2                                   │
│  %doc "Calculate risk score"                │
│  %disallow impure                           │
│  ---                                        │
│  expression                                 │
│  ---                                        │
│  tests (optional)                           │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│  Parser (Pest Grammar)                      │
│  - Extract directives                       │
│  - Parse expression                         │
│  - Parse tests (if present)                 │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│  Directive Processor                        │
│  - Merge with host defaults                 │
│  - Validate against frozen values           │
│  - Check strictness constraints             │
│  - Build configuration object               │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│  Type Checker                               │
│  - Enforce effect restrictions              │
│  - Validate version compatibility           │
│  - Use configuration for checking           │
└─────────────────────────────────────────────┘
```

### Data Model

#### Directive AST Nodes

```rust
/// Represents a parsed directive
#[derive(Debug, Clone)]
pub enum Directive {
    Version(u32),                    // %melbi 2
    Doc(String),                     // %doc "text"
    Allow(Vec<DirectiveFlag>),       // %allow errors, impure
    Disallow(Vec<DirectiveFlag>),    // %disallow errors
    Experimental(Vec<String>),       // %experimental pattern-matching
}

/// Flags that can be allowed/disallowed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectiveFlag {
    Errors,    // Unhandled error effects
    Impure,    // Non-deterministic operations
}

/// Parsed structure of a complete Melbi source
#[derive(Debug)]
pub struct MelbiSource {
    pub directives: Vec<Directive>,
    pub expression: Expr,
    pub tests: Option<Vec<Test>>,
    // Future: pub properties: Option<Vec<Property>>, for formal verification
}
```

**Note on structure**: Currently three sections (directives, expression, tests). Future formal verification features might add a fourth section for properties, or properties could remain as directives. See "Looking into the Future > Formal Verification" for discussion.

#### Configuration Object

```rust
/// Runtime configuration built from directives
#[derive(Debug, Clone)]
pub struct MelbiConfig {
    pub version: Option<u32>,
    pub documentation: Option<String>,
    pub allow_errors: bool,
    pub allow_impure: bool,
    pub experimental_features: HashSet<String>,
}

impl MelbiConfig {
    /// Merge user directives with host defaults
    pub fn from_directives(
        directives: &[Directive],
        host_defaults: &HostConfig,
    ) -> Result<Self, ConfigError> {
        // Validate and merge logic
    }
}
```

#### Host Configuration API

```rust
/// Host-side configuration for Melbi engine
pub struct HostConfig {
    defaults: HashMap<String, DirectiveValue>,
    frozen: HashMap<String, DirectiveValue>,
    relaxation_allowed: HashSet<String>,
}

impl HostConfig {
    pub fn new() -> Self;

    /// Set default value for a directive
    pub fn set_default(&mut self, key: &str, value: DirectiveValue);

    /// Freeze a directive to prevent user changes
    pub fn freeze(&mut self, key: &str, value: DirectiveValue);

    /// Allow users to relax a restriction (rare)
    pub fn allow_relaxation(&mut self, key: &str);
}

#[derive(Debug, Clone)]
pub enum DirectiveValue {
    Allow,
    Disallow,
}
```

### Interface / API Definitions

#### Parser Interface

```rust
/// Parse a complete Melbi source with directives
pub fn parse_source(input: &str) -> Result<MelbiSource, ParseError>;

/// Parse just the directives section
pub fn parse_directives(input: &str) -> Result<Vec<Directive>, ParseError>;
```

#### Type Checker Interface

```rust
/// Type check with configuration
pub fn type_check(
    expr: &Expr,
    config: &MelbiConfig,
    env: &TypeEnv,
) -> Result<Type, TypeError>;

/// Validate effect usage against config
fn validate_effects(
    expr_type: &Type,
    config: &MelbiConfig,
) -> Result<(), TypeError> {
    if !config.allow_errors && expr_type.has_error_effect() {
        return Err(TypeError::UnhandledError {
            suggestion: "Add `%allow errors` or handle with `otherwise`",
        });
    }
    if !config.allow_impure && expr_type.has_impure_effect() {
        return Err(TypeError::ImpureNotAllowed {
            suggestion: "Add `%allow impure` or remove impure operations",
        });
    }
    Ok(())
}
```

### Business Logic

#### Strictness Enforcement

The key invariant: **users can only make directives MORE restrictive, never LESS restrictive.**

```
Host Default    User Directive    Result
─────────────────────────────────────────────
allow           allow             ✓ (no-op)
allow           disallow          ✓ (stricter)
disallow        disallow          ✓ (no-op)
disallow        allow             ✗ (ERROR: cannot relax)
```

Exception: Host can call `allow_relaxation("flag")` to permit specific relaxations.

#### Directive Precedence

1. Start with host defaults
2. Apply user directives in order (last wins for conflicts)
3. Validate against frozen values
4. Check strictness constraints
5. Build final configuration

```rust
fn merge_directives(
    user: &[Directive],
    host: &HostConfig,
) -> Result<MelbiConfig, ConfigError> {
    let mut config = MelbiConfig::from_defaults(host);

    for directive in user {
        match directive {
            Directive::Allow(flags) => {
                for flag in flags {
                    if host.is_frozen(flag) && !host.allows_value(flag, DirectiveValue::Allow) {
                        return Err(ConfigError::FrozenValue { flag });
                    }
                    if !config.can_relax(flag) && host.is_stricter(flag) {
                        return Err(ConfigError::CannotRelax { flag });
                    }
                    config.set_allow(flag);
                }
            }
            Directive::Disallow(flags) => {
                for flag in flags {
                    if host.is_frozen(flag) && !host.allows_value(flag, DirectiveValue::Disallow) {
                        return Err(ConfigError::FrozenValue { flag });
                    }
                    // Disallow is always valid (stricter)
                    config.set_disallow(flag);
                }
            }
            // ... other directives
        }
    }

    Ok(config)
}
```

### Migration Strategy

**Phase 1: Optional Directives (Current)**
- All directives optional
- Existing expressions work unchanged
- New expressions can use directives for clarity

**Phase 2: Recommended Directives**
- Linter/tooling suggests adding `%melbi` version
- Documentation encourages `%doc` for team expressions
- IDE provides directive snippets

**Phase 3: Required Version (Future)**
- After first breaking change, `%melbi <version>` becomes required
- Parser rejects expressions without version
- Clear migration path with error messages

**Phase 4: Feature Gates**
- New features require `%experimental` flag
- After stabilization, feature becomes standard
- Old expressions continue working (no flag needed for stable features)

### Work Required

#### Parser (Pest Grammar)
- Add directive rules: `directive = { "%" ~ directive_type }`
- Add section separator: `section_sep = { "---" }`
- Modify `main` rule to support three sections
- Handle comma-separated and newline-separated lists

#### AST
- Add `Directive` and `DirectiveFlag` enums
- Add `MelbiSource` struct with sections
- Update `ParsedExpr` to include directives

#### Directive Processor (New Module)
- Implement `MelbiConfig` builder
- Implement `HostConfig` API
- Strictness validation logic
- Conflict detection and resolution

#### Type Checker Integration
- Accept `MelbiConfig` parameter
- Validate effects against config
- Generate helpful error messages with directive suggestions

#### Host API
- Public `HostConfig` API for Rust users
- C FFI wrappers for host configuration
- Documentation and examples

#### Error Messages
- New error types for directive conflicts
- Suggestions for resolving issues
- Display current vs required configuration

#### Documentation
- Language reference for directives
- Host integration guide
- Migration guide for version updates
- Examples for common patterns

### Work Sequence

1. **Parser & AST** (Foundation)
   - Add directive grammar rules
   - Parse into AST nodes
   - Unit tests for parsing

2. **Configuration System** (Core Logic)
   - Implement `MelbiConfig` and `HostConfig`
   - Strictness enforcement logic
   - Unit tests for merging/validation

3. **Type Checker Integration** (Enforcement)
   - Pass config to type checker
   - Validate effects
   - Generate directive-aware errors

4. **Host API** (External Interface)
   - Rust API design
   - C FFI wrappers
   - Documentation

5. **Tooling** (Developer Experience)
   - LSP support for directives
   - Linter rules
   - IDE snippets

6. **Tests & Documentation** (Polish)
   - Integration tests
   - Language reference
   - Migration guides

### High-level Test Plan

#### Parser Tests
- Valid directive syntax
- Invalid directive syntax
- Multiple directives
- Comma-separated vs newline-separated
- Section separator variations
- Missing sections

#### Configuration Tests
- Default merging
- Strictness enforcement (allow → disallow ✓, disallow → allow ✗)
- Frozen value validation
- Relaxation permissions
- Conflict detection

#### Type Checker Integration Tests
- Unhandled error with `disallow errors`
- Impure operation with `disallow impure`
- Proper error messages with directive suggestions
- Version compatibility checks

#### End-to-End Tests
- Complete expressions with all directive types
- Host configuration scenarios
- Migration from no-directives to versioned
- Experimental feature enablement

### Deployment Sequence

1. **Core implementation** (parser, config, type checker)
2. **Host API** (Rust first, then C FFI)
3. **Language reference documentation**
4. **LSP integration** (tooling support)
5. **Migration guide** (for future version requirement)

Not user-facing until core implementation is complete and tested.

## Impact

### Performance Impact

**Positive:**
- No runtime overhead (all directive processing at compile time)
- Enables better optimization (pure expressions can be constant-folded)

**Negative:**
- Slightly longer parse time (negligible, ~1-2% increase)
- Additional validation step before type checking

**Mitigation:**
- Directive processing is O(n) in number of directives (typically <10)
- Caching of `MelbiConfig` objects when expressions are reused

### Security Impact

**Positive:**
- Hosts can enforce security policies via frozen directives
- Explicit error handling reduces crash risk
- Determinism guarantee (`disallow impure`) for auditable logic

**Negative:**
- None identified - directives only restrict, never expand capabilities

### Developer Experience Impact

**Positive:**
- Self-documenting expressions via `%doc`
- Team can enforce standards (`disallow impure` for determinism)
- Clear version compatibility
- Better error messages with directive suggestions

**Negative:**
- Learning curve for directive syntax
- Potential for directive overuse (every expression doesn't need them)

**Mitigation:**
- Excellent documentation with examples
- IDE support (autocomplete, snippets)
- Linter warns about unnecessary directives

### Cost Analysis

Development cost:
- ~2-3 weeks for core implementation
- ~1 week for host API and FFI
- ~1 week for documentation and tooling
- ~1 week for testing and polish

Total: ~5-6 weeks of development effort

Maintenance cost:
- Minimal - system is designed to be stable
- New directives can be added incrementally
- No runtime infrastructure needed

## Alternatives

### Alternative 1: No Directives (Status Quo)

**Approach:** Keep everything host-configured, no in-expression metadata.

**Pros:**
- Simplest implementation
- No new syntax to learn
- Smallest language surface area

**Cons:**
- No versioning mechanism for breaking changes
- No self-documentation
- Cannot express user preference for strictness
- Difficult to evolve language safely

**Verdict:** Rejected - need versioning for long-term viability.

### Alternative 2: Traditional `use` Statements

**Approach:** Use statement-based syntax like most languages:
```
use version 2;
use strict;

expression
```

**Pros:**
- Familiar to most developers
- Clear statement boundaries

**Cons:**
- Breaks "expression-only" philosophy
- Requires statement/expression distinction in parser
- More verbose

**Verdict:** Rejected - conflicts with expression-focused design.

### Alternative 3: JSON/YAML Frontmatter

**Approach:** Use structured frontmatter:
```yaml
---
melbi: 2
doc: "Description"
allow: [errors, impure]
---
expression
```

**Pros:**
- Structured data, easy to parse
- Familiar from Markdown/Jekyll

**Cons:**
- Heavier syntax for simple cases
- Requires YAML parser
- Mixing indentation-sensitive and insensitive syntax

**Verdict:** Rejected - too heavyweight for typical use.

### Alternative 4: Pragma Comments

**Approach:** Use comments for directives:
```
// @melbi 2
// @doc "Description"
// @allow errors

expression
```

**Pros:**
- Doesn't require new syntax
- Backwards compatible (ignored by old parsers)

**Cons:**
- Comments are usually for humans, not semantics
- Harder to parse reliably
- Conflicts with actual documentation comments

**Verdict:** Rejected - directives have semantic meaning, shouldn't be comments.

### Alternative 5: Keyword Arguments to Expression

**Approach:** Make directives function-like:
```
melbi(
  version: 2,
  allow: [errors],
  body: expression
)
```

**Pros:**
- Pure expression syntax
- No new grammar needed

**Cons:**
- Confusing scope (is `melbi` a function?)
- Awkward for long expressions
- Doesn't look like configuration

**Verdict:** Rejected - semantically confusing.

## Looking into the Future

### Potential New Directives

**Security/Sandboxing:**
```melbi
%sandbox network: false, filesystem: false
%max recursion: 1000
```

**Type System Extensions:**
```melbi
%experimental union-types
%experimental pattern-matching
```

**Performance Hints:**
```melbi
%optimize aggressive     // Max constant folding
%inline functions        // Aggressive inlining
```

**Testing Integration:**
```melbi
%test coverage: 80%      // Require test coverage
%property quickcheck     // Property-based testing
```

### Version Evolution

As language evolves:
- Melbi 2: Current design
- Melbi 3: Might add pattern matching, require `%melbi 3`
- Melbi 4: Union types become standard

Old expressions continue working in compatibility mode based on `%melbi` version.

### Tooling Integration

- **LSP**: Autocomplete directives, validate against host config
- **Linter**: Suggest appropriate directives, warn about missing docs
- **Formatter**: Standardize directive ordering and formatting
- **Package Manager**: Express dependencies via directives (future)

### Test Section Enhancement

Current design acknowledges test section structure (see `docs/design/unit-testing.md` for full specification). Tests use regular Melbi data structures:

```melbi
%melbi 2
%disallow impure
---
expression
---
[{
    name = "test case name",
    values = { x = 10, y = 5 },
    expected = 15,
}]
```

Future testing enhancements:
- Property-based testing support
- Coverage requirements via `%test coverage: 80%` directive
- Integration with host test frameworks

### Formal Verification and CBMC Integration

Building on the existing test infrastructure, Melbi could integrate formal verification tools like **CBMC (C Bounded Model Checker)** to provide mathematical proofs of correctness. This would create a **three-tier safety model**:

**Tier 1: Type Safety** (Always On)
- Effect tracking (`!` for errors, `~` for impure)
- No implicit conversions
- Pattern matching exhaustiveness

**Tier 2: Example-Based Testing** (Opt-In via Test Section)
- Concrete test cases validate specific inputs
- Fast feedback during development
- Specified in third section after `---`

**Tier 3: Formal Verification** (Opt-In via Directives)
- Mathematical proofs over *all possible inputs*
- Catches edge cases tests might miss
- Specified via `%verify` and `%property` directives

#### Proposed Verification Directives

```melbi
%melbi 2
%doc "Loan approval logic - safety critical"
%disallow errors, impure
%verify no-errors, deterministic, bounds
%property "forall x: x >= 0 => result >= 0"
---
applicant.creditScore * 0.6 + applicant.income * 0.4
---
tests...
```

**Verification directives:**

- `%verify <checks>` - Enable specific verification checks
  - `no-errors` - Prove no runtime errors (division by zero, array bounds, etc.)
  - `deterministic` - Prove expression always returns same output for same input
  - `bounds` - Prove all values stay within expected ranges
  - `overflow` - Prove no integer overflow
  - `termination` - Prove all recursion terminates

- `%property <assertion>` - Assert mathematical properties that must hold
  - Uses quantifiers: `forall`, `exists`
  - Can reference expression inputs and outputs
  - Multiple properties can be specified

- `%bounded depth: N, unroll: M` - Control verification bounds
  - `depth` - Maximum recursion depth to verify
  - `unroll` - Maximum loop iterations to unroll

#### Example: Verified Division

```melbi
%melbi 2
%disallow errors
%verify no-errors
%property "forall x, y: y != 0 => result == x / y"
---
(numerator / denominator) otherwise 0 where {
    denominator = if divisor == 0 then 1 else divisor
}
---
tests...
```

CBMC would prove:
1. No division by zero can occur (enforced by `otherwise` and conditional)
2. Result matches expected division when divisor is non-zero
3. Fallback value is returned when divisor is zero

#### Integration Strategy

**Host API:**
```rust
// Host enables verification
let mut config = HostConfig::new();
config.enable_verification(true);
config.set_verification_timeout(Duration::from_secs(30));

// Run verification during CI/CD
let verification_results = engine.verify(expression, &config)?;
if !verification_results.all_passed() {
    return Err("Formal verification failed");
}
```

**CI/CD Workflow:**
1. Run type checking (instant)
2. Run example-based tests (seconds)
3. Run formal verification if `%verify` present (seconds to minutes)
4. Deploy only if all three pass

#### Use Cases for Formal Verification

**Safety-Critical Systems:**
- Automotive (ISO 26262)
- Aerospace (DO-178C)
- Medical devices (IEC 62304)
- Railway systems (EN 50128)

**High-Stakes Business Logic:**
- Financial risk calculations
- Pricing algorithms
- Smart contracts
- Access control policies

**Regulatory Compliance:**
- Proving absence of specific error conditions
- Demonstrating deterministic behavior for auditing
- Documenting safety properties for certification

#### Synergy with Melbi's Design

CBMC integration is particularly powerful for Melbi because:

1. **Expression-only design** - No statements means simpler control flow to verify
2. **Effect system** - `~` impure tracking enables verification of determinism
3. **No runtime errors** - Type system + `otherwise` + CBMC = mathematical proof
4. **Embedded use case** - Safety-critical embeddings benefit most from verification
5. **Arena allocation** - Bounded memory model aligns with CBMC's bounded verification

This would make Melbi **the first embeddable expression language with integrated formal verification** - a unique selling point for safety-critical applications.

#### Implementation Considerations

**Phase 1: Foundation**
- Add `%verify`, `%property`, `%bounded` directives to grammar
- Design API for verification backend integration
- Implement simple properties (no division by zero)

**Phase 2: CBMC Integration**
- Translate Melbi expressions to C code CBMC can verify
- Map Melbi types to C types with assertions
- Handle effect system in translation

**Phase 3: Advanced Properties**
- Support quantified properties (`forall`, `exists`)
- Verify recursive functions with termination proofs
- Generate counter-examples for failed properties

**Future: Alternative Backends**
- Z3 SMT solver for theorem proving
- Dafny for verification-first approach
- Coq for proof assistants

**Estimated effort:** 8-12 weeks for Phase 1, additional 12-16 weeks for Phase 2 with CBMC

### Multi-Expression Projects

Currently single-expression focused. Future might support:
- Shared directive files (like `.melbirc`)
- Directive inheritance
- Project-level defaults
- Module system (if needed)

---

## Appendix: Syntax Examples

### Minimal Expression
```melbi
10 + 20
```
No directives needed for simple cases.

### Documented Expression
```melbi
%doc "Calculate monthly payment for loan"

principal * (rate * (1 + rate)^months) / ((1 + rate)^months - 1)
```

### Strict Expression
```melbi
%melbi 2
%disallow errors
%disallow impure

applicant.creditScore > 650 otherwise false
```

### With Experimental Features
```melbi
%melbi 2
%experimental pattern-matching

status match {
    Approved -> "Welcome",
    Pending -> "Please wait",
    Rejected -> "Sorry"
}
```

### Complete with Tests
```melbi
%melbi 2
%doc "Risk score calculator"
%disallow impure
---
score = creditScore * 0.4 + income * 0.0001 + years * 5
---
test "high credit score" {
    creditScore = 800
    income = 50000
    years = 10
    expect = 375
}
```

### Multiple Directives (Variations)
```melbi
// Comma-separated
%allow errors, impure

// Separate lines
%allow errors
%allow impure

// Mixed
%allow errors
%disallow impure
```

All equivalent and valid.

### With Formal Verification (Future)
```melbi
%melbi 2
%doc "Safety-critical calculation for autonomous vehicle"
%disallow errors, impure
%verify no-errors, deterministic, bounds, overflow
%property "forall speed, distance: distance > 0 => result > 0"
---
stoppingTime = distance / speed where {
    speed = if velocity <= 0 then 1 else velocity
}
---
[{
    name = "normal speed",
    values = { velocity = 50, distance = 100 },
    expected = 2,
}, {
    name = "zero speed handled",
    values = { velocity = 0, distance = 100 },
    expected = 100,
}]
```

This expression demonstrates all three safety tiers:
1. **Type safety**: Effect system ensures errors are handled
2. **Example testing**: Concrete test cases validate behavior
3. **Formal verification**: CBMC proves safety for all inputs
