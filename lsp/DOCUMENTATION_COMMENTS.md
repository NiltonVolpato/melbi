# Documentation Comments Design

## Overview

This document describes the design for adding documentation comment support to Melbi. Since Melbi is an embedded expression language, documentation is particularly important for understanding what host-provided functions and values do.

## Goals

1. Allow documentation of:
   - Host-provided globals (functions, constants, packages)
   - Where-clause bindings within expressions
   - Lambda parameters (future)

2. Display documentation in IDE hover tooltips

3. Support both:
   - Runtime-provided documentation (from host Rust code)
   - Inline documentation (for where-clause bindings)

## Doc Comment Syntax

### For Where Bindings (Inline)

```melbi
result where {
    /// Calculates the sum of two numbers
    sum = x + y,

    /// The base value for calculations
    x = 10,

    y = 20,
    result = sum * 2
}
```

### For Host-Provided Globals (Runtime)

Host code would provide documentation when registering globals:

```rust
// In host Rust code
let globals = vec![
    Global {
        name: "Stats.Sum",
        ty: function_type,
        doc: Some("Calculates the sum of all numbers in an array"),
    },
];
```

## Implementation Plan

### Phase 1: Infrastructure (Current)

✅ Add placeholder for docs in hover response
✅ Design document (this file)
⬜ Add `doc: Option<&str>` field to globals parameter in analyzer
⬜ Thread docs through to TypedExpr somehow

### Phase 2: Parser Changes

⬜ Modify Pest grammar to capture doc comments:
   ```pest
   DOC_COMMENT = ${ "///" ~ (!"\n" ~ ANY)* }
   ```

⬜ Store doc comments in AnnotatedSource alongside spans

⬜ Associate doc comments with the following where-binding

### Phase 3: Analyzer Integration

⬜ Extend TypeScheme or create a new wrapper type:
   ```rust
   struct DocumentedBinding<'types> {
       scheme: TypeScheme<'types>,
       doc: Option<&'arena str>,
   }
   ```

⬜ Thread documentation through scope stack

⬜ Make documentation available in TypedExpr

### Phase 4: LSP Integration

⬜ Extract docs from TypedExpr in hover handler

⬜ Format docs as Markdown in hover response:
   ```rust
   fn format_hover(type_str: &str, doc: Option<&str>) -> String {
       let mut result = format!("```melbi\n{}\n```", type_str);
       if let Some(doc) = doc {
           result.push_str("\n\n---\n\n");
           result.push_str(doc);
       }
       result
   }
   ```

⬜ Support multiple paragraphs, code examples in docs

## Example Hover Output

### Without Documentation
```
Int
```

### With Documentation
```
Stats.Sum :: [Number] -> Number

---

Calculates the sum of all numbers in an array.

**Example:**
```melbi
Stats.Sum([1, 2, 3, 4, 5])  // Returns: 15
```
```

## Challenges

### 1. Parser Integration

Comments are currently silent rules (_) in Pest, which means they're discarded. We need to:
- Make doc comments non-silent (but still skip regular comments)
- Capture them during parsing
- Associate them with the next significant token

### 2. Lifetime Management

Documentation strings need to live as long as the expression. Options:
- Store in the arena (best for inline docs)
- Store in a separate map keyed by expression pointer
- Include in the AnnotatedSource structure

### 3. Multi-line Comments

Should we support `/* ... */` style doc comments?
```melbi
/**
 * Calculates the sum of two numbers.
 * Returns the result as an integer.
 */
sum = x + y
```

Recommendation: Start with `///` only, add `/** */` later if needed.

## API Design

### For Host Rust Code

```rust
// Providing documentation for globals
let globals = vec![
    (
        "Stats.Sum",
        sum_type,
        Some("Calculates the sum of all numbers in an array")
    ),
];

engine.analyze(source, &globals, &variables)?;
```

### For LSP Hover

```rust
impl DocumentState {
    pub fn hover_at_position(&self, position: Position) -> Option<HoverInfo> {
        // ... find expression at position ...

        Some(HoverInfo {
            type_str: format!("{}", expr.type_),
            documentation: expr.documentation, // NEW!
            range: Some(expr_range),
        })
    }
}
```

## Testing Strategy

1. **Unit tests** for doc comment parsing
2. **Integration tests** for documentation in hover
3. **Manual tests** in VS Code/Zed

Test cases:
- Doc comment before where binding
- Multiple doc comments
- Doc comment with blank lines
- Doc comment with code examples
- Host-provided documentation
- No documentation (fallback to type only)

## Migration Path

Since this is a new feature, there's no migration needed. The changes are:
1. Backward compatible (old code without docs still works)
2. Opt-in (hosts don't need to provide docs)
3. Additive (doesn't break existing features)

## Future Enhancements

1. **Markdown rendering** in hover (bold, italic, code blocks)
2. **Link resolution** (e.g., `See [Stats.Mean]`)
3. **Parameter documentation** for lambdas
4. **Return value documentation** separate from description
5. **Examples section** with runnable code
6. **"See also" section** linking to related functions

## Notes

- Documentation is purely for developer experience in IDEs
- Does not affect runtime behavior or type checking
- Should be extracted and shown by LSP, not stored in compiled expressions
- Consider using a dedicated `Documentation` type to ensure consistent formatting
