# Option[T] Type and Pattern Matching Implementation Plan

**Branch**: `feature/option-type-pattern-matching`
**Started**: 2025-11-16

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

### Phase 2: Add Constructor Expressions (Some/None)
- [ ] Add constructor grammar to `parser/expression.pest`
- [ ] Add `Constructor` variant to `Expr` in `parser/parsed_expr.rs`
- [ ] Implement constructor parsing in `parser/parser.rs`
- [ ] Add `Constructor` to `ExprInner` in `analyzer/typed_expr.rs`
- [ ] Implement constructor type checking in `analyzer/analyzer.rs`
- [ ] Add `Variant` to `RawValue` union in `values/dynamic.rs`
- [ ] Implement constructor evaluation in `evaluator/eval.rs`
- [ ] Write tests for `Some(42)`, `None`, type inference

**Details**: (To be expanded when starting this phase)

---

### Phase 3: Add Match Expressions
- [ ] Add match grammar to `parser/expression.pest`
- [ ] Add `Match`, `MatchCase`, `Pattern` types to `parser/parsed_expr.rs`
- [ ] Implement pattern parsing in `parser/parser.rs`
- [ ] Add `Match` to `ExprInner` in `analyzer/typed_expr.rs`
- [ ] Implement match type checking in `analyzer/analyzer.rs`
- [ ] Implement pattern evaluation in `evaluator/eval.rs`
- [ ] Write tests for basic pattern matching

**Details**: (To be expanded when starting this phase)

---

### Phase 4: Exhaustiveness Checking
- [ ] Create new `analyzer/exhaustiveness.rs` module
- [ ] Implement exhaustiveness checker for Option type
- [ ] Integrate with match expression analysis
- [ ] Add warnings for unreachable patterns
- [ ] Write tests for coverage checking

**Details**: (To be expanded when starting this phase)

---

## Design Decisions

### Key Architectural Choices
- **Specialized Option type**: Using `Option(&'a Type<'a>)` instead of generic Union for null-pointer optimization and common-case performance
- **No unwrap()**: Pattern matching is the only safe way to extract Option values
- **Lowercase keywords**: `some` and `none` (not `Some`/`None`) following Melbi's keyword style
- **Prefix operator**: `some` works like unary `-` at same precedence level
- **Polymorphic none**: Like empty array `[]`, `none` has type `Option[_0]`
- **VM already supports pattern-matching**: Leverage existing pattern-matching opcodes
- **Phased approach**: Each phase is independently testable

### Dependencies
- Option type needed for useful pattern matching
- Pattern matching needed to avoid unwrap()
- Therefore: implemented together

---

## Notes
- VM instruction set already has pattern matching opcodes (MatchBegin, MatchConstructor, etc.)
- "match" keyword already reserved in grammar
- Generic types (Array[T], Map[K,V]) provide blueprint for Option[T]
- Type inference/unification infrastructure already in place
