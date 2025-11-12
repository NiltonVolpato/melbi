# Melbi Language Support for Visual Studio Code

Provides rich language support for
[Melbi](https://github.com/NiltonVolpato/melbi), an embeddable
expression-focused programming language designed to be flexible, type-safe,
fast, efficient, and sandboxed.

## Features

✅ **Syntax Highlighting** - TextMate grammar-based syntax highlighting
✅ **Type Checking** - Real-time type error detection with Hindley-Milner inference
✅ **Code Formatting** - Automatic code formatting with `melbi-fmt`
✅ **Hover Information** - View type information on hover
✅ **Diagnostics** - Inline error and warning messages
✅ **Auto-completion** - Context-aware code suggestions (basic)

## Installation

### From Source

1. Clone the repository:
   ```bash
   git clone https://github.com/NiltonVolpato/melbi.git
   cd melbi/vscode
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Compile the extension:
   ```bash
   npm run compile
   ```

4. Install in VS Code:
   - Press `F5` to open a new VS Code window with the extension loaded
   - Or package with `vsce package` and install the `.vsix` file

### Requirements

- Visual Studio Code 1.100.0 or higher
- The Melbi language server (`melbi-lsp`) must be built and available in your PATH

To build the language server:
```bash
cd melbi
cargo build --release --package melbi-lsp
# Add target/release/melbi-lsp to your PATH
```

## Language Server Features

This extension uses the Melbi Language Server Protocol (LSP) implementation:

- **Syntax errors** from tree-sitter parser
- **Type errors** from the Melbi type checker with precise error locations
- **Type information** in hover tooltips (shows inferred types)
- **Auto-formatting** using Topiary formatter
- **Auto-completion** for basic constructs (expanding)

## File Extensions

The extension recognizes these file extensions:
- `.melbi`
- `.mb`
- `.🖖` (Vulcan salute emoji!)

## Example Code

```melbi
// Simple arithmetic
1 + 2 * 3

// Variables with where clauses
result where {
    x = 10,
    y = 20,
    result = x * y,
}

// Lambda functions
((x) => x * 2)(5)

// Arrays and higher-order functions
[1, 2, 3, 4, 5]

// Records
{ name = "Melbi", version = "0.1.0" }

// Conditional expressions
if x > 10 then "large" else "small"

// Format strings
f"Hello, {name}!"

// Calling host functions (defined in Rust)
Stats.Sum([1, 2, 3, 4, 5])
```

## Commands

- **Format Document** - Right-click and select "Format Document" or use `Shift+Alt+F`

## Extension Settings

Currently, this extension contributes no VS Code settings. Configuration coming in future releases.

## Known Limitations

- **Go-to-Definition**: Not implemented, as Melbi is an embedded language where most definitions come from the host Rust code
- **Hover positioning**: Currently shows type for entire expression; position-aware hover coming soon
- **Completion**: Basic infrastructure in place, scope-based completions coming soon
- **Multi-file workspaces**: Currently analyzes files independently

## Future Features

🔜 Position-aware hover with exact expression types
🔜 Documentation comments in hover tooltips
🔜 Scope-based auto-completion
🔜 Code snippets for common patterns
🔜 Inlay hints showing inferred types
🔜 Semantic syntax highlighting

## Troubleshooting

### Language server not starting

1. Ensure `melbi-lsp` is built:
   ```bash
   cargo build --release --package melbi-lsp
   ```

2. Check it's in your PATH:
   ```bash
   which melbi-lsp  # Unix/Mac
   where melbi-lsp  # Windows
   ```

3. Check the Output panel in VS Code (View → Output → Melbi Language Server)

### Syntax highlighting not working

The syntax highlighting uses a TextMate grammar. If it's not working, try:
1. Reload VS Code window (Cmd/Ctrl+R)
2. Check the file extension is `.melbi`, `.mb`, or `.🖖`

## Development

The language server source code is in the `lsp/` directory of the main Melbi repository.

To contribute:
1. Make changes to the LSP server in `lsp/`
2. Rebuild: `cargo build --release --package melbi-lsp`
3. Restart the extension in VS Code to test

## Issues & Contributing

Found a bug? Have a feature request?
Please open an issue at: https://github.com/NiltonVolpato/melbi/issues

## License

MIT OR Apache-2.0

---

**Enjoy using Melbi!** 🖖
