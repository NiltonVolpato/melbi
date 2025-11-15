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

## Recommended Solution

**Solution 1 (Inline Specialized Lambdas)** is recommended because:

1. **Correctness:** Each call site has explicit, concrete types
2. **Performance:** Substitution happens once during type checking, not at runtime
3. **Future-proof:** Enables monomorphization for compilation
4. **Clear semantics:** The expression tree explicitly shows what's happening

### Implementation Strategy

1. Add a post-processing phase after `finalize_constraints()`:
   ```rust
   fn specialize_polymorphic_calls(expr: TypedExpr) -> TypedExpr
   ```

2. Walk the expression tree looking for patterns:
   - `Call(Ident(name), args)` where `name` is bound to a polymorphic lambda
   - `Where { bindings: [...], expr: ... }` to find polymorphic bindings

3. For each polymorphic call:
   - Extract the call's concrete types from unification
   - Build substitution map from type scheme to concrete types
   - Walk lambda body tree and substitute type variables
   - Replace `Ident(name)` with specialized `Lambda{...}`

4. This produces a tree where:
   - All type variables are resolved
   - All polymorphic lambdas are inlined and specialized
   - Array construction sees consistent concrete types

## Open Questions

1. **Where exactly should specialization happen?**
   - After `finalize_constraints()` in the `analyze()` function?
   - As a separate pass in the caller?
   - Should it be optional (for interpretation vs compilation)?

2. **How do we extract concrete types from call sites?**
   - The call expression has type `_5`, but we need `Str`
   - Do we need to walk through unification one more time?
   - Or should we build a "final substitution map" during `finalize_constraints()`?

3. **How do we handle nested polymorphic calls?**
   - What if a polymorphic lambda calls another polymorphic lambda?
   - Do we need multiple passes, or can we handle it in one traversal?

4. **Should we keep the `where` bindings in the tree?**
   - After inlining, the bindings are unused
   - Remove them? Keep them for debugging?
   - Does this affect closure capture semantics?

5. **Performance considerations:**
   - Is tree copying too expensive for large lambda bodies?
   - Should we share common subtrees?
   - Do we need arena allocation for the new trees?

6. **Monomorphization limits:**
   - Should we limit the number of unique instantiations?
   - What happens with recursive polymorphic functions?
   - How do we detect infinite instantiation?

## Related Code

- `core/src/analyzer/analyzer.rs`: Type checking and expression tree construction
- `core/src/types/unification.rs`: Type variable instantiation
- `core/src/types/type_class_resolver.rs`: Constraint copying (`copy_constraints_with_subst`)
- `core/src/evaluator/eval.rs`: Expression evaluation (where the problem manifests)

## Current Status

- Type checking for polymorphic map indexing **works correctly**
- Constraint copying applies full substitution (fixed in commit 9d42da9)
- Evaluator fails on array construction due to unresolved type variables
- Need to implement one of the solutions above to fix evaluation
