# Polymorphic Lambda Type Resolution Problem

## Problem Description

When polymorphic lambdas are stored in `where` bindings and later called with different type instantiations, the evaluator receives expression trees with stale type variables that don't match the instantiated types.

### Example

```
[f({1: "one"}, 1), f({"one": "uno"}, "one")] where { f = (m, k) => m[k] }
```

**Expected behavior:** The polymorphic function `f` should work with both `Map[Int, Str]` and `Map[Str, Str]`.

**What happens:**

1. **Analysis phase:**
   - Lambda `(m, k) => m[k]` is analyzed with type variables `_0`, `_1`, `_2`
   - Body expression tree: `Expr(_2, Index { value: Expr(_0, Ident("m")), index: Expr(_1, Ident("k")) })`
   - Generalized to: `∀[0,1,2]. (_0, _1) -> _2` with constraint `Indexable(_0, _1, _2)`

2. **First call `f({1: "one"}, 1)`:**
   - Type checking instantiates to fresh variables: `(_3, _4) -> _5`
   - Constraint copied: `Indexable(_3, _4, _5)`
   - Unification: `_3 = Map[Int, Str]`, `_4 = Int`, `_5 = Str`
   - Call expression gets type `_5`
   - **But evaluator looks up lambda and gets body with `_0, _1, _2`!**

3. **Second call `f({"one": "uno"}, "one")`:**
   - Type checking instantiates to: `(_7, _8) -> _9`
   - Unification: `_7 = Map[Str, Str]`, `_8 = Str`, `_9 = Str`
   - Call expression gets type `_9`
   - **Again, evaluator gets body with `_0, _1, _2`!**

4. **Array construction:**
   - Array element types: `_5` and `_9`
   - After final unification: both `_5` and `_9` resolve to `Str`
   - **But the expressions were allocated with unresolved type variables**
   - Array constructor sees `Expr(_5, ...)` and `Expr(_9, ...)` as different types
   - Fails with: "Array construction failed - analyzer should have validated types"

## Root Cause

The issue has two parts:

### 1. Stale Type Variables in Lambda Bodies

When a polymorphic lambda is stored in a `where` binding:
- The **type** gets generalized (stored as `TypeScheme`)
- The **expression tree** keeps the original type variables

When the lambda is later instantiated:
- The **type** gets fresh variables (e.g., `_3, _4, _5`)
- The **expression tree** is reused with old variables (e.g., `_0, _1, _2`)

The evaluator needs the types in the expression tree to be consistent with the instantiated types, but they're not.

### 2. Premature Type Resolution in `alloc()`

The `alloc()` method calls `fully_resolve()` on types during expression construction:

```rust
fn alloc(&mut self, ty: &'types Type<'types>, inner: ExprInner<'types, 'arena>) -> ... {
    let typed_expr = self.arena.alloc(Expr(self.unification.fully_resolve(ty), inner));
    ...
}
```

However, unification continues **after** expressions are allocated:
- Call expression allocated with type `_5` (unresolved)
- Later, `_5` gets unified with `Str`
- But the allocated expression still has type `_5` in the tree
- Final array construction sees different type variables for elements that should have the same type

## Possible Solutions

### Solution 1: Inline Specialized Lambdas at Call Sites

**Approach:** During a final resolve pass, when we encounter `Call(Ident("f"), args)` where `f` is polymorphic:
1. Look up the lambda definition from the binding
2. Extract concrete types from the call site (e.g., `Map[Int, Str]`, `Int`, `Str`)
3. Build substitution map (e.g., `{0→Map[Int,Str], 1→Int, 2→Str}`)
4. Walk the lambda body tree and substitute all type variables
5. Replace with `Call(Lambda{specialized_body}, args)`

**Result:**
```rust
Array[
  Call(Lambda{body with Map[Int,Str], Int, Str}, [...]),
  Call(Lambda{body with Map[Str,Str], Str, Str}, [...])
]
```

**Pros:**
- Each call site has explicit types - no runtime substitution needed
- Enables **monomorphization** for bytecode/machine code generation
- Easy to identify all unique instantiations of a polymorphic function
- Evaluator sees regular lambdas with concrete types

**Cons:**
- Code duplication in the expression tree
- Need to walk and copy lambda body trees
- Increases memory usage for trees with many instantiations

**Monomorphization benefits:**
For bytecode/machine code generation, you can:
1. Collect all unique instantiations during the resolve pass
2. Generate specialized code for each: `f<Map[Int,Str],Int,Str>`, `f<Map[Str,Str],Str,Str>`
3. At call sites, emit call to appropriate specialization
4. No runtime type checking or substitution needed

### Solution 2: Store Substitution in Ident Nodes

**Approach:** When instantiating a polymorphic identifier:
1. Add `subst` field to `ExprInner::Ident`: `Option<HashMap<u16, &'types Type>>`
2. In `analyze_ident`, store the instantiation substitution
3. In evaluator, when looking up an `Ident` with substitution, apply it to the lambda body

**Pros:**
- Smaller tree size - substitution stored once per call site
- Clear where instantiation happens

**Cons:**
- Evaluator must walk and substitute trees at runtime (expensive)
- Makes evaluator aware of type substitution logic
- Runtime overhead on every polymorphic call

### Solution 3: Final Resolve Pass on Entire Tree

**Approach:** After `finalize_constraints()`, walk the entire result expression tree once and replace all type variables with their fully resolved types.

**Pros:**
- Simple - just one pass over the final tree
- All types become concrete (e.g., `_5` → `Str`, `_9` → `Str`)
- Fixes the array construction issue

**Cons:**
- **Doesn't fix the lambda body problem** - the lambda in the `where` binding still has `_0, _1, _2` which aren't unified with anything
- Can't distinguish between different instantiations
- Not suitable for monomorphization/bytecode generation

### Solution 4: Store Untyped AST and Re-analyze

**Approach:** Store untyped AST for polymorphic functions and re-analyze on each instantiation.

**Pros:**
- Guaranteed fresh, correct types
- No tree walking/substitution needed

**Cons:**
- Very expensive - full type checking on every call
- Defeats the purpose of type schemes

## Recommended Solution: Hybrid Approach

The problem has **two independent issues** that require **two different mechanisms**:

### Issue A: Lambda Body Type Variables
- Lambda bodies retain original type variables (`_0, _1, _2`)
- Call sites use fresh instantiated variables (`_3, _4, _5`)
- **Solution:** Track instantiation info for polymorphic lambdas

### Issue B: Expression Tree Type Variables
- Call expressions allocated with type `_5` before unification completes
- Array elements have types `_5` and `_9` which later both unify to `Str`
- But the tree still shows `Expr(_5, ...)` and `Expr(_9, ...)`
- **Solution:** Final type resolution pass after `finalize_constraints()`

### Why Both Are Needed

**Instantiation tracking alone is NOT sufficient** because:
- It only helps with lambda body variables (`_0, _1, _2`)
- Doesn't fix type variables throughout the rest of the tree (`_5`, `_9`, etc.)
- Array construction, record fields, and other type comparisons still fail

**Type resolution pass alone is NOT sufficient** because:
- Lambda body variables (`_0, _1, _2`) are never unified with anything
- They're part of the generalized type scheme, not the unification graph
- `fully_resolve()` can't resolve them - they're not constraints to be solved
- Evaluator/bytecode generator needs to know the concrete instantiation

### Recommended Hybrid Approach

**Phase 1: Final Type Resolution Pass**
- After `finalize_constraints()`, walk the entire expression tree
- Replace all type variables with their fully resolved types: `_5 → Str`, `_9 → Str`
- Fixes array construction and all type comparisons
- Makes the tree ready for evaluation

**Phase 2: Track Instantiation Information**
- During type checking, when instantiating a polymorphic lambda:
  - Record: `lambda_id → instantiation_subst`
  - Store all unique instantiations for each polymorphic lambda
- Attach this information to the analyzed result
- Evaluator/bytecode generator uses it to:
  - Apply substitutions to lambda bodies at runtime (interpreter)
  - Generate specialized code for each instantiation (compiler)

### Implementation Strategy

**Part 1: Type Resolution**
```rust
fn resolve_all_type_variables(expr: TypedExpr, unification: &Unification) -> TypedExpr {
    // Walk tree and replace all type variables with fully_resolve()
}
```

**Part 2: Instantiation Tracking**
```rust
struct InstantiationInfo<'types> {
    lambda_expr: TypedExpr<'types>,  // The original lambda with _0, _1, _2
    instantiations: Vec<HashMap<u16, &'types Type<'types>>>,  // All observed substitutions
}

struct AnalysisResult<'types> {
    expr: TypedExpr<'types>,
    instantiations: HashMap<LambdaId, InstantiationInfo<'types>>,
}
```

**Part 3: Evaluator Usage**
- When evaluating `Call(Ident("f"), args)`:
  - Look up lambda from `where` binding (has type variables `_0, _1, _2`)
  - Look up instantiation from `instantiations` map
  - Apply substitution to lambda body before executing

**Part 4: Bytecode Generator Usage**
- For each lambda with multiple instantiations:
  - Generate specialized bytecode for each unique instantiation
  - Map call sites to appropriate specialization
  - Enables true monomorphization

## Open Questions

### Architecture

1. **Where should type resolution happen?**
   - After `finalize_constraints()` in the `analyze()` function?
   - As a separate pass in the caller?
   - Should analyzer return `AnalysisResult` with both `expr` and `instantiations`?

2. **Where should instantiation tracking happen?**
   - In `analyze_ident` when looking up polymorphic identifiers?
   - In `instantiate()` method in `Unification`?
   - How do we identify which lambda a call refers to?

3. **Lambda identification:**
   - How do we uniquely identify lambdas for the instantiation map?
   - Use expression pointer address?
   - Add explicit `LambdaId` to `ExprInner::Lambda`?
   - What about lambdas that aren't bound to names?

### Implementation Details

4. **Type resolution mechanics:**
   - How do we walk the expression tree to replace types?
   - Do we allocate new `Expr` nodes or mutate in place?
   - Arena allocation means we can't mutate - need to copy tree?

5. **Instantiation deduplication:**
   - How do we detect duplicate instantiations?
   - Hash the substitution map? Compare types structurally?
   - Does order matter (`{_0→Int, _1→Str}` vs `{_1→Str, _0→Int}`)?

6. **Evaluator integration:**
   - How does evaluator get access to `InstantiationInfo`?
   - Pass it as parameter to `eval()`?
   - Store it in evaluator state?
   - How to match call site to instantiation?

### Edge Cases

7. **Nested polymorphic calls:**
   - What if a polymorphic lambda calls another polymorphic lambda?
   - Do we track instantiations for both?
   - Can the inner lambda's instantiation depend on the outer's?

8. **Recursive polymorphic functions:**
   - What happens with recursive calls?
   - Can we have infinite unique instantiations?
   - Should we limit instantiation depth?

9. **Higher-order functions:**
   - What if a polymorphic lambda is passed as an argument?
   - How do we track its instantiation at the call site?
   - Does the receiving function need to know the instantiation?

### Performance

10. **Memory overhead:**
    - Is storing all instantiations too expensive?
    - For large programs with many calls, could grow significantly
    - Should we have a limit on unique instantiations per lambda?

11. **Bytecode generation:**
    - Should bytecode generator inline specialized versions?
    - Or keep runtime type substitution?
    - Trade-off: code size vs execution speed

## Related Code

- `core/src/analyzer/analyzer.rs`: Type checking and expression tree construction
- `core/src/types/unification.rs`: Type variable instantiation
- `core/src/types/type_class_resolver.rs`: Constraint copying (`copy_constraints_with_subst`)
- `core/src/evaluator/eval.rs`: Expression evaluation (where the problem manifests)

## Current Status

- Type checking for polymorphic map indexing **works correctly**
- Constraint copying applies full substitution (fixed in commit 9d42da9)
- Evaluator fails on array construction due to unresolved type variables
- Need to implement the hybrid approach described above

## Summary and Next Steps

To fix the polymorphic lambda evaluation issue, we need **both mechanisms**:

1. **Type Resolution Pass** (required immediately):
   - Fixes array construction and type comparisons
   - Makes the tree ready for evaluation
   - Simple to implement: walk tree and call `fully_resolve()` on all type references

2. **Instantiation Tracking** (required for correctness):
   - Fixes lambda body type variable mismatch
   - Enables evaluator to apply correct substitutions
   - Enables bytecode generator to perform monomorphization
   - More complex: requires design decisions about lambda identification and data structures

**Priority:** Implement type resolution pass first to get tests passing, then add instantiation tracking to properly handle polymorphic lambda bodies.

**Answer to the question:** Yes, we still need a second pass to resolve type variables throughout the expression tree, even with instantiation tracking. The two mechanisms solve different problems and are both necessary.
