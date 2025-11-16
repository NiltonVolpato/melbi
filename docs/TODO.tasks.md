# TODO Tasks

Random tasks we want to do - some sooner, some later.

**Priorities**: P0 (critical) → P1 (high) → P2 (medium) → P3 (low)  
_Re-evaluate priorities periodically as needs change_

---

## Error Handling

- [ ] **Store source code in Error struct and simplify render_error API** (P2)
  - Currently `render_error(source, &error)` requires passing source separately
  - Should store source in `Error` during compilation so API becomes `render_error(&error)`
  - Benefits: Simpler API, can't accidentally pass wrong source
  - Related files: `core/src/api/error.rs`, `src/error_renderer.rs`

- [ ] **Design FFI error handling strategy** (P2)
  - FFI functions should have their own error type (separate from Melbi's Error)
  - FFI trait should include function name/metadata for better error messages
  - When FFI error propagates into Melbi, explicit conversion adds source/span context
  - Conversion point: where evaluator calls FFI function - knows the call site span
  - This allows FFI errors to be simple (no source/span) while Melbi errors remain rich
  - Benefits: FFI errors stay lightweight, conversion adds context at the right boundary
  - Related files: `core/src/api/error.rs`, evaluator FFI bridge code, FFI trait definition

- [ ] **Integrate error objects from melbi-core** (P2)
  - Currently Analyzer returns `Error` enum and Evaluator returns `EvalError` enum
  - Should unify these into a single error type or create a clear hierarchy
  - Related files: `core/src/errors.rs`, `core/src/evaluator/error.rs`
  - This would improve error handling consistency across the codebase

- [ ] **Improve recursive closure error messages** (P3)
  - Currently recursive closures are impossible (analyzer correctly rejects them)
  - Error message could be more helpful: "Undefined variable 'factorial'" should suggest that recursive functions need special support
  - Test case: `factorial(5) where { factorial = (n) => if n <= 1 then 1 else n * factorial(n - 1) }`
  - Current behavior: Analyzer reports "Undefined variable 'factorial'" because the lambda body is analyzed before the binding is complete
  - Better error: "Recursive closures are not supported. Consider using a named function instead."
  - Related: Phase 4 Milestone 4.1 in lambda-closure-implementation-plan.md
  
- [ ] **Add RecursiveClosure and UndefinedVariable error variants** (P3)
  - Deferred from lambda implementation - handle as part of broader error reporting improvements
  - `RecursiveClosure` variant for when lambda references itself
  - `UndefinedVariable` variant as defensive error (should be caught by analyzer)
  - These would provide better error messages than generic type checking errors
  - Related files: `core/src/evaluator/error.rs`

---

## Features

- [ ] **Implement map indexing evaluation** (P0)
  - **CRITICAL**: Currently crashes - no crashes are acceptable
  - Add support for evaluating map indexing operations like `map[key]`
  - Type checking already supports the `Indexable` type class for maps
  - Need to implement the evaluation logic in the evaluator
  - Verify if analyzer tests exist for map indexing type checking
  - Related files: `core/src/evaluator/mod.rs`, `core/src/types/type_class.rs`
  - Should handle both valid keys and missing key errors gracefully

---

## Notes

- Use checkboxes `- [ ]` for tasks that can be tracked
- Add context and related files to help future work
- Move completed tasks to the bottom with `- [x]` or remove them
