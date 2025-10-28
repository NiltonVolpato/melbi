# Melbi Formatter Instructions

## Code Formatting Workflow

- **Topiary-based** formatter with custom Melbi grammar
- **Idempotency testing**: `topiary` tests for idempotency automatically:
  ```bash
  printf 'code' | topiary format --language melbi
  ```
  To debug failures use `--skip-idempotence` flag.
- Query file: `topiary-queries/queries/melbi.scm`

## Topiary Formatting Queries

When working with Topiary formatting queries in `topiary-queries/queries/melbi.scm`:

1. **Always check the documentation first**: The topiary reference documentation is available in `ref/topiary-book/src/reference/`. Key files:
   - `capture-names/scopes.md` - Understanding scoped vs non-scoped softlines
   - `capture-names/vertical-spacing.md` - Hardlines and softlines
   - `capture-names/indentation.md` - Indentation rules

2. **Scoped vs Non-Scoped Softlines**:
   - **Non-scoped** (`@append_spaced_softline`, `@prepend_spaced_softline`): Check if the **immediate parent CST node** is multi-line
   - **Scoped** (`@append_spaced_scoped_softline`, `@prepend_spaced_scoped_softline`): Check if the **custom scope** (defined with `&num;scope_id!`) is multi-line
   - Use **scoped** softlines when you need to control formatting based on what's *inside* a custom scope, not based on external context

## Testing Formatter Changes

When modifying the Topiary queries or formatter logic:

1. **Test idempotency**: Run the formatter twice on the same input and verify identical output
2. **Check existing tests**: Run `cargo test` to ensure all formatting expectations still pass
3. **Add new test cases**: Use the `test_case!` macro with `formatted` expectations for new syntax
4. **Debug formatting issues**: Use `--skip-idempotence` flag when debugging query problems

## Multi-Line Formatted Expectations

When writing `formatted` expectations for multi-line output in `test_case!` macros:

- Use raw strings (`r&num;"..."&num`) starting with a newline for readability in code
- **Important**: Place the raw string at the beginning of the line (first column) since raw strings preserve all whitespace, including indentation.
- Apply `.trim_start()` to remove the leading newline, ensuring the string content matches the formatter's output exactly
- Include trailing newlines in the raw string **only if the input ends with a newline** (the formatter preserves input trailing newlines)

Example:

```rust
formatted: r&num"
[
    1,
    2,
    3,
]"&num.trim_start(),
```

Example with input ending in newline:

```rust
test_case!(
    multi_line_with_newline,
    input: indoc! {"
        [1,
         2]
"},  // ends with newline
    formatted: r&num"
[
    1,
    2,
]
"&num.trim_start(),  // includes trailing newline
);
```

Example with input NOT ending in newline:

```rust
test_case!(
    multi_line_no_newline,
    input: indoc! {"
        [1,
         2]"},  // does NOT end with newline
    formatted: r&num"
[
    1,
    2,
]"&num.trim_start(),  // does NOT include trailing newline
);
```

This convention ensures test expectations are readable while accurately matching the formatter's behavior.