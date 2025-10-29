---
title: Melbi Test System Design
---

# Design Doc: Unit Testing for Melbi Expressions

**Author**: @NiltonVolpato

**Date**: 10-29-2025

## Introduction

### Background

Melbi is an embeddable expression language designed for safe evaluation of user-provided code. Users write expressions that get deployed to production systems (e.g., email filters, feature flags, business rules). Currently, there is no built-in way for users to validate their expressions work as intended before deployment.

**Problem**: Users can't easily verify their expressions produce expected results for various inputs. This leads to:
- Broken expressions in production
- Time-consuming manual testing
- Low confidence when writing complex logic
- Difficulty catching regressions when context schema changes

**Solution**: Add a built-in test system that allows users to write test cases alongside their expressions. Tests validate that expressions produce expected outputs for given inputs.

**Stakeholders**:
- End users writing Melbi expressions
- Application developers embedding Melbi
- CI/CD systems that need to validate expressions before deployment

### Current Functionality

Currently, Melbi has:
- Expression parser and type checker
- Type-safe evaluation with effect system
- `where` clause for local bindings
- No testing capabilities

### In Scope

This design addresses:
- Syntax for defining test cases within Melbi source files
- Grammar modifications to support test sections
- Type checking test cases against expression types
- API for executing tests and reporting results
- Deployment workflow (stripping tests for production)

### Out of Scope

- Testing expressions that intentionally error (may be added later with `expect error` syntax)
- Code coverage analysis
- Performance benchmarking
- Test generation or property-based testing
- Interactive test debugging tools

### Assumptions & Dependencies

- Parser uses Pest grammar (easy to extend)
- Type system can validate test contexts match schema
- Expression evaluation already supports binding arbitrary contexts
- Tests execute in CI/CD before production deployment
- Production systems receive expression source without tests

### Terminology

- **Test case**: A record containing test name, input values, and expected output
- **Test section**: The portion of source code after `---` containing test definitions
- **Test mode compilation**: Compilation that preserves global function references (allows override)
- **Production mode compilation**: Fully optimized compilation with inlining
- **Context schema**: Type definitions for variables available to expressions

## Considerations

### Concerns

**Syntax complexity**: Adding test syntax must feel natural within Melbi's expression-only design. We avoid statement-like constructs that don't exist elsewhere in the language.

**Type safety**: Test inputs and expected outputs must be type-checked to catch errors early. Type mismatches in tests should produce clear error messages.

**Performance**: Tests should not impact production compilation or runtime performance. Test execution should be fast enough for CI/CD workflows.

**User experience**: Test syntax should be intuitive for users familiar with testing frameworks, while remaining consistent with Melbi's style.

### Operational Readiness Considerations

**Deployment**: Tests are stripped from source before production deployment (simple text operation removing everything after `---`). This is done at the CI/CD layer, not by the Melbi compiler.

**Validation**: CI/CD pipelines run `melbi test` command before deployment. Failed tests block deployment.

**Debugging**: Test failures report:
- Test case name
- Expected value
- Actual value
- Input context

**No operational metrics needed**: Tests run only in development/CI environments, not production.

### Open Questions

1. **Error testing syntax**: How should users test expressions that should produce errors? Potential syntax: `expected = error` or `expected = Err("message")`. This is deferred for now since Melbi requires error handling via `otherwise`.

2. **Function equality**: Functions can't be compared for equality. Should tests that expect function values be rejected, or should we implement pointer equality? **Decision**: Accept as limitation; tests expecting functions will fail at runtime with clear error message.

3. **Test mode compilation overhead**: How much optimization should be disabled in test mode? Need to benchmark to find balance between allowing overrides and maintaining reasonable test performance.

4. **Array concatenation operator**: The design uses `++` for combining test suites, but this operator may change to `+`. The test syntax is agnostic to this choice.

### Cross-Region Considerations

Not applicable - tests execute in development/CI environments only, not in production across regions.

## Proposed Design

### Solution

Tests are expressed as regular Melbi data structures - an array of records. Each record contains a test name, input values, and expected output. Tests are separated from the main expression using a YAML-style document separator (`---`). This approach reuses existing language features (`where` clause, records, arrays) rather than introducing special-purpose test syntax.

**Key insight**: By making tests just data, we get test composition, grouping, and shared contexts "for free" using standard Melbi features like `where` and array concatenation.

**Emergent capability**: Parameterized and generated tests require no special framework support. Since tests are just arrays of records, users can generate them using any array manipulation features in Melbi (list comprehensions, `map`, `filter`, etc.).

### System Architecture

Components involved:

1. **Parser** (`melbi-core`): Extended to recognize test sections
2. **Type checker** (`melbi-core`): Validates test case types
3. **Evaluator** (`melbi-core`): Executes tests with provided contexts
4. **CLI** (`melbi-cli`): Provides `melbi test` command
5. **CI/CD tools**: Strip tests and run validation before deployment

```
┌─────────────────────────────────────────┐
│  Source File (expression + tests)      │
└──────────────┬──────────────────────────┘
               │
               ▼
         ┌──────────┐
         │  Parser  │  Separates expression from tests
         └─────┬────┘
               │
         ┌─────▼──────────────────────┐
         │  Type Checker              │
         │  - Check expression type   │
         │  - Check test types        │
         │  - Validate contexts       │
         └─────┬──────────────────────┘
               │
         ┌─────▼──────────────────────┐
         │  Test Runner               │
         │  - Compile in test mode    │
         │  - Evaluate each test      │
         │  - Compare results         │
         └─────┬──────────────────────┘
               │
         ┌─────▼──────────────────────┐
         │  Results                   │
         │  - Pass/Fail per test      │
         │  - Detailed failure info   │
         └────────────────────────────┘
```

### Data Model

**Test case record structure**:
```melbi
{
    name: String,
    values: Record[...],    // Bindings for test context
    expected: T,            // Expected result (type T matches expression type)
}
```

**Test section type**: `Array[TestCase[T]]` where `T` is the main expression's return type.

**Example AST representation** (conceptual):
```rust
pub struct ParsedProgram<'a> {
    pub expression: &'a Expr<'a>,
    pub tests: Option<&'a Expr<'a>>,  // Evaluates to Array[TestCase]
}
```

### Interface / API Definitions

**Parsing API**:
```rust
pub fn parse<'a>(
    arena: &'a Bump,
    source: &str,
) -> Result<ParsedProgram<'a>, ParseError>;
```

**Type checking API**:
```rust
pub fn type_check<'a>(
    program: &ParsedProgram<'a>,
    schema: &ContextSchema,
) -> Result<Type<'a>, TypeError> {
    // Returns the expression's type
    // Also validates all test cases
}
```

**Test execution API**:
```rust
pub fn run_tests<'a>(
    program: &ParsedProgram<'a>,
    schema: &ContextSchema,
) -> Result<TestResults, TestError>;

pub struct TestResults {
    pub total: usize,
    pub passed: usize,
    pub failed: Vec<TestFailure>,
}

pub struct TestFailure {
    pub test_name: String,
    pub expected: Value,
    pub actual: Value,
    pub input_context: HashMap<String, Value>,
}
```

**CLI interface**:
```bash
# Run tests
melbi test expression.mb

# Run tests with custom schema
melbi test expression.mb --schema schema.json

# Check types only (don't execute)
melbi check expression.mb
```

### Business Logic

**Test execution algorithm**:

1. Parse source into expression + test section
2. Type-check expression against schema
3. Evaluate test section expression to get array of test case records
4. For each test case:
   a. Type-check `values` record matches schema
   b. Type-check `expected` value matches expression type
   c. Compile expression in test mode (no global inlining)
   d. Evaluate expression with test's `values` as context
   e. Compare result with `expected` using structural equality
   f. Record pass/fail
5. Return aggregate results

**Type checking logic**:

```rust
fn type_check_tests(
    tests_expr: &Expr,
    schema: &ContextSchema,
    expected_type: &Type,
) -> Result<(), TypeError> {
    // Tests expression must evaluate to Array[Record[...]]
    let tests_type = infer_type(tests_expr)?;

    match tests_type {
        Type::Array(elem_type) => {
            match elem_type {
                Type::Record(fields) => {
                    // Check required fields: name, values, expected
                    check_field_type(fields, "name", Type::String)?;
                    check_field_type(fields, "values", schema.to_record_type())?;
                    check_field_type(fields, "expected", expected_type)?;
                }
                _ => return Err(TypeError::InvalidTestStructure),
            }
        }
        _ => return Err(TypeError::TestsMustBeArray),
    }

    Ok(())
}
```

**Equality comparison**: Use structural equality for all types except functions. Functions throw runtime error if compared in tests.

### Test Generation and Parameterization

Since tests are regular data structures (arrays of records), users can generate parameterized tests using standard language features - no special test framework support needed.

**Simple parameterized tests** (using list operations):

```melbi
calculate_discount(tier)

---

// Generate tests for different tiers
[
    {tier = "bronze", discount = 0.05},
    {tier = "silver", discount = 0.10},
    {tier = "gold", discount = 0.20},
    {tier = "platinum", discount = 0.30},
].map((tc) => {
    name = "calculates " ++ tc.tier ++ " discount",
    values = {tier = tc.tier},
    expected = tc.discount,
})
```

*Note: The actual syntax may use list comprehensions (Python-style or Haskell-style) rather than `.map()`. Either approach allows generating tests programmatically.*

**Cross-product testing**: When testing combinations of inputs, users can generate all combinations using nested list comprehensions or similar constructs. This is particularly useful for testing operations with multiple parameters across different input ranges.

For example, testing a math operation across multiple operators and edge cases would naturally generate the full cross-product of test cases without requiring special framework features like "parameterized test matrices."

### Migration Strategy

No migration needed - this is a new feature. Existing Melbi source files without test sections continue to work unchanged.

**Adoption path**:
1. Users can start adding tests to new expressions immediately
2. Existing expressions can have tests added incrementally
3. CI/CD pipelines can enforce test requirements via compilation flags

### Work Required

**Phase 1: Grammar and Parsing** (~2-3 days)
- Add `test_separator` and `test_section` to Pest grammar
- Modify parser to recognize optional test section after expression
- Update `ParsedProgram` structure to include optional tests
- Add test cases for parser

**Phase 2: Type Checking** (~3-4 days)
- Implement test case record type validation
- Add validation that `values` matches context schema
- Add validation that `expected` matches expression type
- Ensure helpful error messages for type mismatches
- Add test cases for type checker

**Phase 3: Test Execution** (~4-5 days)
- Implement test mode compilation (disable global inlining)
- Implement test runner that evaluates each test case
- Implement structural equality comparison
- Build `TestResults` and `TestFailure` reporting
- Add test cases for evaluator

**Phase 4: CLI Integration** (~2-3 days)
- Add `melbi test` command to CLI
- Add test result formatting and output
- Add exit codes for CI/CD integration
- Add test cases for CLI

**Phase 5: Documentation** (~2-3 days)
- Write user guide for test syntax
- Document CI/CD integration patterns
- Add examples for common test scenarios
- Update language reference

**Total estimate**: ~15-20 days of focused development

**No external dependencies** - all work contained within Melbi codebase.

### Work Sequence

1. Grammar and parsing (must be complete before type checking)
2. Type checking (must be complete before execution)
3. Test execution (can be developed in parallel with CLI)
4. CLI integration (depends on test execution)
5. Documentation (can begin once basic functionality works)

### High-level Test Plan

**Unit tests** for each component:
- Parser: Test various test section syntaxes (valid and invalid)
- Type checker: Test type validation for test cases
- Evaluator: Test execution of test cases and equality comparison
- CLI: Test command-line interface and output formatting

**Integration tests**:
- End-to-end test: Write expression with tests, run `melbi test`, verify results
- CI/CD simulation: Test strip-and-deploy workflow
- Error scenarios: Invalid test syntax, type mismatches, runtime failures

**Example test cases** (eating our own dog food):
```melbi
// Test the test system itself!
true

---

[{
    name = "constant true expression",
    values = {},
    expected = true,
}]
```

### Deployment Sequence

1. Merge grammar changes (backwards compatible - old files still parse)
2. Merge type checker changes (backwards compatible - no tests = no validation)
3. Merge test execution (feature complete but not yet exposed)
4. Merge CLI changes (exposes `melbi test` command)
5. Release new version with test support
6. Update documentation and examples
7. Notify users of new testing capabilities

No coordination required with other systems - purely additive feature.

## Impact

### Performance Impact

**Parsing**: Negligible - test section is optional and parsed only when present. Adds ~5-10 lines to grammar.

**Type checking**: Minimal - only validates tests if present. No impact on expressions without tests.

**Compilation**: No impact - tests are never compiled in production mode. Test mode compilation is used only during test execution.

**Runtime**: Zero impact - tests are stripped before deployment. Production systems never see test code.

### Security Impact

**No new security risks**: Tests execute in the same sandbox as expressions. Test contexts are validated by type checker.

**Positive impact**: Tests validate expressions work correctly, reducing risk of logic errors in production.

### Cost Analysis

**Development cost**: ~15-20 days of engineering time.

**Operational cost**: Zero - tests run only in CI/CD, not production.

**Maintenance cost**: Low - test syntax reuses existing language features.

### Cross-Region Considerations

Not applicable - tests don't deploy to production or run across regions.

## Alternatives

### Alternative 1: Special Test Syntax (Rejected)

```melbi
tests {
    test "name" expect value where { ... },
    test "name" expect value where { ... },
}
```

**Why rejected**: Requires wrapping `tests { }` block and special `test` keyword. More syntax to learn. Harder to compose tests or share contexts. No way to generate parameterized tests without additional framework features.

### Alternative 2: Test Cases as Top-Level Items (Rejected)

```melbi
expression

---

test "case 1" expect value where { ... }
test "case 2" expect value where { ... }
```

**Why rejected**: Would require sequence of expressions at top level, breaking Melbi's "single expression" design. Ambiguous where one test ends and next begins.

### Alternative 3: Separate Test Files (Rejected)

Tests in `.test.mb` files that reference the main expression file.

**Why rejected**: Splits related code across files. More complex to manage. Harder to deploy (need to track which test file goes with which expression).

### Alternative 4: Assertion Functions (Rejected)

```melbi
assert(expression == expected, "test name")
```

**Why rejected**: Requires adding side-effectful `assert` function. Doesn't fit with Melbi's pure expression model. Tests would modify expression behavior rather than being separate.

## Looking into the Future

### Immediate Next Steps (Post-Launch)

**Error testing syntax**: Add support for testing expressions that should error:
```melbi
[{
    name = "division by zero",
    values = { x = 10, y = 0 },
    expected = error,  // or: expected = Err("division by zero")
}]
```

**Better error messages**: When tests fail, show diff-style output for complex values (arrays, records).

### Medium-Term Enhancements

**Test coverage analysis**: Track which branches/conditions are exercised by tests. Report untested code paths.

**Property-based testing**: Generate random test inputs and verify properties:
```melbi
property "commutative" {
    forall x, y: Int => x + y == y + x
}
```

**Performance benchmarks**: Time test execution and track performance regression:
```melbi
benchmark "name" {
    values = { ... },
    max_time = 1ms,
}
```

### Long-Term Vision

**Interactive test mode**: REPL-like environment for debugging failing tests. Step through expression evaluation, inspect intermediate values.

**Test generation from examples**: Given input/output pairs, automatically generate test cases.

**Mutation testing**: Automatically modify expression to verify tests catch bugs.

**Integration with external test frameworks**: Export test results in JUnit XML, TAP, or other standard formats for CI/CD integration.

---

**Document Status**: Design complete, ready for implementation
**Last Updated**: October 29, 2025
**Next Review**: After implementation phase 1 (grammar/parsing)
