# GitHub Copilot Instructions

## Project-Specific Guidelines

### Topiary Formatting Queries

When working with Topiary formatting queries in `topiary-queries/queries/melbi.scm`:

1. **Always check the documentation first**: The topiary reference documentation is available in `topiary-book/src/reference/`. Key files:
   - `capture-names/scopes.md` - Understanding scoped vs non-scoped softlines
   - `capture-names/vertical-spacing.md` - Hardlines and softlines
   - `capture-names/indentation.md` - Indentation rules

2. **Scoped vs Non-Scoped Softlines**:
   - **Non-scoped** (`@append_spaced_softline`, `@prepend_spaced_softline`): Check if the **immediate parent CST node** is multi-line
   - **Scoped** (`@append_spaced_scoped_softline`, `@prepend_spaced_scoped_softline`): Check if the **custom scope** (defined with `#scope_id!`) is multi-line
   - Use **scoped** softlines when you need to control formatting based on what's *inside* a custom scope, not based on external context

3. **Idempotency Testing**: Always test formatting idempotency by running the formatter twice:
   ```bash
   printf 'code' | topiary format --language melbi | topiary format --language melbi
   ```
   If the output differs between runs, there's an idempotency violation.

### Testing Infrastructure

- Use the `test_case!` macro from `tests/cases/mod.rs` for declarative test writing
- The macro supports optional fields: `input`, `formatted`, `ast`, `error`
- Use `indoc!` for readable multi-line string literals in tests
- Each test file in `tests/` is compiled as a separate integration test crate

### General Workflow

1. **Documentation first, experimentation second**: When encountering unfamiliar tools or libraries, check for reference documentation before trial-and-error debugging
2. **Read the error messages carefully**: Topiary provides detailed error messages with diffs for formatting issues
3. **Test incrementally**: Run tests frequently to catch issues early
