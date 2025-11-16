# Option[T] Type and Pattern Matching Implementation Plan

**Branch**: `feature/option-type-pattern-matching`
**Started**: 2025-11-16
**Completed**: 2025-11-16

## Overview

Implement `Option[T]` generic type with pattern matching support. No `unwrap()` - pattern matching is the safe way to handle Options.

## Implementation Phases

### Phase 1: Add Option[T] Type to Type System ✅ COMPLETED
- [x] Add sync comment to expression.pest and tree-sitter grammar
- [x] Add `Option(&'a Type<'a>)` variant to `Type` enum (discriminant 11)
- [x] Add `option()` factory method to TypeManager with interning
- [x] Add `Option[T]` type parsing to from_parser.rs
- [x] Add Option unification rules to unification.rs
- [x] Update TypeKind, TypeTag, TypeBuilder, TypeTransformer, TypeVisitor in traits.rs
- [x] Update TypeView implementation for &Type and EncodedType
- [x] Update CompareTypeArgs Hash and PartialEq
- [x] Add encoding/decoding support in encoding.rs
- [x] Add Option cases to manager.rs (adopt_type, alpha_convert)
- [x] Add placeholder support in dynamic.rs (TODOs for runtime representation)
- [x] Add comprehensive tests (15 tests: parsing, display, unification)
- [x] Add `some` and `none` keywords to both grammars
- [x] All tests passing (949 tests)

**Status**: Complete. Option[T] is now fully integrated into the type system infrastructure.

---

### Phase 2: Add Constructor Expressions (Some/None) ✅ COMPLETED

#### Part 1: Grammar (Already Done in Phase 1) ✅
- [x] `some` as prefix operator in expression.pest (line 54, 59)
- [x] `none` as scalar literal in expression.pest (line 171, 180)
- [x] Both added to reserved_words (lines 256-257)
- [x] Tree-sitter grammar updated to match

#### Part 2: AST Structure - ParsedExpr ✅
**File**: `parser/parsed_expr.rs`

- [x] Add `Option` variant to `Expr` enum
- [x] Update `Expr::span()` method to handle Option variant
- [x] Update `Expr::children()` method to return inner expr if Some
- [x] Add Display implementation for Option variant

**Key Decision**: Single `Option` node for both `some expr` and `none`, with `inner: Option<&'a Expr<'a>>` distinguishing them.

#### Part 3: Parser Implementation ✅
**File**: `parser/parser.rs`

- [x] Handle `some_op` prefix in Pratt parser
- [x] Handle `none` literal
- [x] Add tests for parsing

#### Part 4-8: Type Checking, Runtime, and Evaluation ✅
- [x] Add `Option` variant to TypedExpr (typed_expr.rs)
- [x] Implement type checking for Option expressions (analyzer.rs)
- [x] Add `Value::optional()` constructor with null-pointer optimization (dynamic.rs)
- [x] Update Value equality, comparison, hashing, display (dynamic.rs)
- [x] Implement Option evaluation (eval.rs)
- [x] Write comprehensive tests (5+ parser tests, all 970 tests passing)

**Status**: Complete! All 970 tests passing. Option constructors (`some` and `none`) are now fully functional.

---

### Phase 3: Pattern Matching Implementation ✅ COMPLETED

**Goal**: Implement postfix match expressions with exhaustiveness checking for Bool and Option types.

#### Design Decisions

**Syntax**: Postfix `expression match { pattern -> body, ... }`
- Same precedence as `where` (lowest postfix level)
- Consistent with Melbi's design philosophy
- Enables pipeline-style composition
- Match arms use `->` while lambdas use `=>`

**Pattern Types (MVP)**:
1. ✅ Wildcard `_` - matches anything, binds nothing
2. ✅ Variable `x` - matches anything, binds to name
3. ✅ Literals: `1`, `3.14`, `true`, `"hello"`, `b"bytes"`
4. ✅ Option constructors: `some p`, `none`
5. ✅ Nested patterns: `some (some x)`

**Exhaustiveness Checking**:
- ✅ Bool: require `(true, false)` or wildcard
- ✅ Option[T]: require `(some _, none)` or wildcard (recursive for nested)
- ✅ Other types: no checking, require explicit wildcard

**Deferred to Phase 4+**:
- Or-patterns `1 | 2 | 3`
- Ranges `1..10`
- Array destructuring `[a, b, ...]`
- Record patterns `{name = a, ...}`
- Guards `pattern if condition`

#### Implementation Tasks - All Completed ✅

##### Task 1+3: Grammar and Parser ✅
**Files**: `parser/expression.pest`, `parser/parser.rs`

- [x] Add `match_op` to `postfix_op` rule (same level as `where`)
- [x] Define flat pattern grammar with separate Pratt parser
- [x] Fixed pattern grammar ordering (wildcard before variable to parse `_` correctly)
- [x] Add `match_op` to Pratt parser at `where` precedence level
- [x] Implement `parse_match_op()` for postfix match
- [x] Implement `parse_match_arm()` for arm parsing
- [x] Implement pattern Pratt parser (parse_pattern, parse_pattern_primary)
- [x] Create `PATTERN_PRATT` parser instance
- [x] Add parser tests (test_pattern_matching_syntax with 13 test cases)

**Key Fix**: Pattern grammar ordering - `pattern_wildcard` must come before `pattern_var` in grammar alternatives, otherwise `_` parses as a variable name.

##### Task 2: AST Types ✅
**File**: `parser/parsed_expr.rs`

- [x] Add `Match` variant to `Expr`
- [x] Add `MatchArm` struct
- [x] Add `Pattern` enum (Wildcard, Var, Literal, Some, None)
- [x] Update Expr methods (span, children, display)

##### Task 4: Typed AST ✅
**File**: `analyzer/typed_expr.rs`

- [x] Add `Match` variant to `ExprInner`
- [x] Add `TypedMatchArm` struct
- [x] Add `TypedPattern` enum with type information
- [x] Update test helper (collect_lambda_pointers)

##### Task 5: Analyzer Type Checking ✅
**File**: `analyzer/analyzer.rs`

- [x] Implement `analyze_match()` - analyze matched expression and all arms
- [x] Implement `collect_pattern_vars()` - gather pattern variable names
- [x] Implement `analyze_pattern()` - type-check patterns and bind variables
- [x] Handle scope management with `IncompleteScope` for pattern bindings
- [x] Unify arm return types to ensure consistency
- [x] Add type resolution for patterns in `resolve_pattern_types()`

**Critical Bug Fix**: Pattern type inference now uses **unification** instead of pattern matching on types. When analyzing `some x` pattern:
- Before: Pattern matched on `expected_ty.view()` which fails for type variables
- After: Unifies `expected_ty` with `Option[fresh_var]`, enabling inference from pattern usage
- Impact: Lambda parameters can now be inferred from pattern matching: `(x) => x match { some y -> y * 2, none -> 0 }` correctly infers type `(Option[Int]) => Int`

##### Task 6: Exhaustiveness Checking ✅
**File**: `analyzer/analyzer.rs`, `analyzer/error.rs`

- [x] Implement `check_exhaustiveness()` for Bool and Option types
- [x] Check for (true, false) or wildcard for Bool
- [x] Check for (some _, none) or wildcard for Option[T]
- [x] Add `NonExhaustivePatterns` error type with helpful messages
- [x] Error reports missing cases (e.g., "Missing cases: false, none")

##### Task 7: Evaluator ✅
**File**: `evaluator/eval.rs`

- [x] Implement `Match` case in eval_expr_inner
- [x] Implement `match_pattern()` - pattern matching with binding extraction
- [x] Handle all pattern types (Wildcard, Var, Literal, Some, None)
- [x] First-match semantics (try arms in order, first match wins)
- [x] Scope management (push/pop pattern bindings)
- [x] Import TypedPattern type

##### Task 8: Tests ✅
**Files**: `parser/parse_test.rs`, `evaluator/eval_test.rs`

Parser tests:
- [x] 13 comprehensive pattern matching syntax tests
- [x] Variable, wildcard, literal, Option patterns
- [x] Nested patterns, mixed patterns

Evaluator tests (14 new tests):
- [x] test_match_variable_pattern
- [x] test_match_wildcard_pattern
- [x] test_match_literal_int
- [x] test_match_literal_bool_true/false
- [x] test_match_literal_string
- [x] test_match_option_some/none
- [x] test_match_option_nested_some/some_none
- [x] test_match_in_where_binding
- [x] test_match_pattern_order (first match wins)
- [x] test_match_with_expression_in_body
- [x] test_match_in_lambda_with_inferable_type (type inference fix)
- [x] test_match_in_where_with_known_type

#### Success Criteria - All Met ✅

Phase 3 complete when:
- ✅ Postfix match syntax parses correctly
- ✅ All pattern types work (wildcard, variable, literal, some, none)
- ✅ Nested patterns work (`some (some x)`)
- ✅ Type checking validates patterns
- ✅ **Type inference works from pattern usage**
- ✅ Exhaustiveness checking for Bool/Option
- ✅ Pattern matching evaluates correctly
- ✅ All 1000 tests pass (except 1 pre-existing stack depth issue)
- ✅ 27+ new tests pass (13 parser + 14 evaluator)

**Status**: ✅ **COMPLETE!** Pattern matching is fully implemented and production-ready.

#### Notable Implementation Details

**Type Inference Enhancement**:
The implementation includes a significant improvement to type inference. Pattern matching now properly constrains type variables through unification, allowing functions like:
```melbi
(x) => x match { some y -> y * 2, none -> 0 }
```
to correctly infer type `(Option[Int]) => Int` without explicit type annotations.

**Exhaustiveness Checking**:
Provides helpful compile-time errors:
```
Non-exhaustive patterns: match on type 'Bool' does not cover all cases
Missing cases: false
```

**First-Match Semantics**:
Patterns are tried in order, first match wins. This allows patterns like:
```melbi
x match { _ -> 1, 42 -> 2 }  // Always returns 1, wildcard catches all
```

---

### Phase 4: Extended Patterns (Future)
- [ ] Or-patterns: `1 | 2 | 3`
- [ ] Array destructuring: `[a, b, ...]`
- [ ] Rest patterns: `[first, ...rest]`
- [ ] Guards: `pattern if condition`
- [ ] Record patterns: `{name = a, age = b, ...}`
- [ ] Range patterns: `1..10`

**Details**: (To be expanded in future)

---

## Design Decisions

### Key Architectural Choices
- **Specialized Option type**: Using `Option(&'a Type<'a>)` instead of generic Union for null-pointer optimization and common-case performance
- **No unwrap()**: Pattern matching is the only safe way to extract Option values
- **Lowercase keywords**: `some` and `none` (not `Some`/`None`) following Melbi's keyword style
- **Prefix operator**: `some` works like unary `-` at same precedence level
- **Polymorphic none**: Like empty array `[]`, `none` has type `Option[_0]`
- **Postfix match**: `expr match { ... }` enables pipeline-style composition
- **Arrow distinction**: `->` for match arms, `=>` for lambdas
- **Unification-based pattern analysis**: Patterns constrain types through unification, not just checking
- **Phased approach**: Each phase is independently testable

### Dependencies
- Option type needed for useful pattern matching
- Pattern matching needed to avoid unwrap()
- Therefore: implemented together in coordinated phases

---

## Final Statistics

**Total Implementation Time**: ~4 hours
**Lines of Code Added**: ~800
**Tests Added**: 27 (13 parser + 14 evaluator)
**Tests Passing**: 1000/1001 (99.9%)
**Type Inference Enhancement**: Pattern-based type constraint propagation

---

## Notes
- VM instruction set already has pattern matching opcodes (MatchBegin, MatchConstructor, etc.) - ready for future use
- "match" keyword already reserved in grammar
- Generic types (Array[T], Map[K,V]) provided blueprint for Option[T]
- Type inference/unification infrastructure supported pattern implementation
- Wildcard `_` parsing required careful grammar ordering
- Type inference fix enables sophisticated type constraint propagation
