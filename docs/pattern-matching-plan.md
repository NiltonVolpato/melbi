# Option[T] Type and Pattern Matching Implementation Plan

**Branch**: `feature/option-type-pattern-matching`
**Started**: 2025-11-16

## Overview

Implement `Option[T]` generic type with pattern matching support. No `unwrap()` - pattern matching is the safe way to handle Options.

## Implementation Phases

### Phase 1: Add Union/Option Type to Type System
- [ ] Add `Union(&'a [&'a Type<'a>])` variant to `Type` enum in `types/types.rs`
- [ ] Add `union()` and `option()` factory methods to `TypeManager`
- [ ] Add `Option[T]` parsing support in `types/from_parser.rs`
- [ ] Add Union unification rules in `types/unification.rs`
- [ ] Update `TypeKind` in `types/traits.rs` to include Union
- [ ] Write tests for Option type creation and unification

**Details**: (To be expanded when starting this phase)

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
- **Union vs specialized Option**: Using general Union type for extensibility (supports future sum types)
- **No unwrap()**: Pattern matching is the only safe way to extract Option values
- **VM already supports patterns**: Leverage existing pattern matching opcodes
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
