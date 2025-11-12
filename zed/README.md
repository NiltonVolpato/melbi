# Melbi Language Support for Zed

Provides rich language support for
[Melbi](https://github.com/NiltonVolpato/melbi), an embeddable
expression-focused programming language designed to be flexible, type-safe,
fast, efficient, and sandboxed.

## Features

✅ **Syntax Highlighting** - Full tree-sitter based syntax highlighting
✅ **Type Checking** - Real-time type error detection with Hindley-Milner inference
✅ **Code Formatting** - Automatic code formatting with `melbi-fmt`
✅ **Hover Information** - View type information on hover
✅ **Diagnostics** - Inline error and warning messages
✅ **Document Symbols** - Navigate code structure with outline view

## Installation

### From Source

1. Clone this repository
2. Navigate to the `zed/` directory
3. Install the extension in Zed

### Requirements

- Zed editor
- The Melbi language server will be automatically downloaded when the extension activates

## Language Server Features

This extension uses the Melbi Language Server Protocol (LSP) implementation, which provides:

- **Syntax errors** from tree-sitter parser
- **Type errors** from the Melbi type checker
- **Type information** in hover tooltips
- **Auto-formatting** using Topiary

## File Extensions

The extension recognizes these file extensions:
- `.melbi`
- `.mb`
- `.🖖` (Vulcan salute emoji!)

## Example Code

```melbi
// Simple arithmetic
1 + 2 * 3

// With variables
x * 2 where { x = 5 }

// Lambda functions
((x) -> x + 1)(5)

// Type-safe operations
Stats.Sum([1, 2, 3, 4, 5])
```

## Known Limitations

- **Go-to-Definition**: Not implemented, as Melbi is an embedded language where definitions often come from the host Rust code
- **Hover positioning**: Currently shows type for entire expression; position-aware hover coming soon
- **Completion**: Basic infrastructure in place, full implementation coming soon

## Development

The language server source code is in the `lsp/` directory of the main Melbi repository.

To rebuild the language server:
```bash
cargo build --release --package melbi-lsp
```

## Issues & Contributing

Found a bug? Have a feature request?
Please open an issue at: https://github.com/NiltonVolpato/melbi/issues

## License

MIT OR Apache-2.0
