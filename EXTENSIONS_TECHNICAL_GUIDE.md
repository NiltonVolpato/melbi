# Melbi Extensions: Detailed Technical Recommendations

## 1. Critical Bug Fix: Zed Grammar Path

### Current Issue
**File:** `/home/user/melbi/zed/extension.toml` (line 10-11)

```toml
[grammars.melbi]
repository = "file:///Users/nilton/Code/tree-sitter-melbi"  # HARDCODED DEV PATH!
```

### Impact
- Extension will not work for any user except Nilton
- No fallback to GitHub repository
- Blocks production use

### Recommended Fix

**Option 1: Use GitHub URL (Recommended)**
```toml
[grammars.melbi]
repository = "https://github.com/NiltonVolpato/tree-sitter-melbi"
rev = "main"  # or specific tag like "v0.1.0"
```

**Option 2: Support Both Dev and Production**
```toml
[grammars.melbi]
# Try local development path first, fall back to GitHub
repository = "file:///Users/nilton/Code/tree-sitter-melbi"
fallback_repository = "https://github.com/NiltonVolpato/tree-sitter-melbi"
rev = "HEAD"
```

---

## 2. VS Code Extension Enhancement: Activation Events

### Current Issue
**File:** `/home/user/melbi/vscode/package.json`

No `activationEvents` specified means the extension loads on startup regardless of whether user is working with Melbi files.

### Recommended Fix

```json
{
  "name": "melbi",
  "displayName": "Melbi Language Support",
  "version": "0.1.0",
  "publisher": "nilton-volpato",
  "engines": { "vscode": "^1.100.0" },
  "activationEvents": [
    "onLanguage:melbi"
  ],
  "contributes": {
    "languages": [{
      "id": "melbi",
      "aliases": ["Melbi", "melbi"],
      "extensions": [".mb", ".melbi", ".🖖"],
      "configuration": "./language-configuration.json",
      "icon": {
        "light": "./icons/melbi-light.png",
        "dark": "./icons/melbi-dark.png"
      }
    }],
    "grammars": [{
      "language": "melbi",
      "scopeName": "source.melbi",
      "path": "./syntaxes/melbi.tmLanguage.json"
    }],
    "debuggers": [{
      "type": "melbi-debug",
      "label": "Melbi",
      "program": "./out/debugger.js",
      "runtime": "node",
      "configurationAttributes": {
        "launch": {
          "required": ["program"],
          "properties": {
            "program": {
              "type": "string",
              "description": "Path to Melbi program to debug"
            },
            "stopOnEntry": {
              "type": "boolean",
              "description": "Stop on entry"
            }
          }
        }
      }
    }]
  }
}
```

**Benefits:**
- Extension loads only when Melbi files are opened
- Faster VS Code startup
- Reduced memory footprint

---

## 3. LSP Enhancement: Implement Go-to-Definition

### Current State
Definition provider is commented out in `/home/user/melbi/lsp/src/main.rs` (lines 58-59)

### Implementation Steps

#### Step 1: Update initialization in main.rs

```rust
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncKind::FULL),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string()]),
                    ..Default::default()
                }),
                document_formatting_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),  // NEW!
                references_provider: Some(OneOf::Left(true)),   // NEW!
                document_symbol_provider: Some(OneOf::Left(true)), // NEW!
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "Melbi Language Server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            ..Default::default()
        })
    }

    // Add new handler
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let location = {
            self.documents
                .get(&uri)
                .and_then(|doc| doc.definition_at_position(position))
        };

        Ok(location.map(GotoDefinitionResponse::Scalar))
    }

    async fn references(
        &self,
        params: ReferenceParams,
    ) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let references = {
            self.documents
                .get(&uri)
                .map(|doc| doc.references_at_position(&self.documents, position))
                .unwrap_or_default()
        };

        Ok(Some(references))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        let symbols = {
            self.documents
                .get(&uri)
                .map(|doc| doc.document_symbols())
                .unwrap_or_default()
        };

        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }
}
```

#### Step 2: Extend DocumentState in document.rs

```rust
impl DocumentState {
    pub fn definition_at_position(&self, position: Position) -> Option<Location> {
        use melbi_core::{analyzer, parser, types::manager::TypeManager};

        if !self.type_checked {
            return None;
        }

        let arena = Bump::new();
        let parsed = parser::parse(&arena, &self.source).ok()?;
        let type_manager = TypeManager::new(&arena);

        let _typed_expr = analyzer::analyze(
            type_manager,
            &arena,
            parsed,
            &[],
            &[],
        ).ok()?;

        // TODO: Implement span-based lookup in typed AST
        // This would require maintaining a mapping of positions to AST nodes
        None
    }

    pub fn references_at_position(
        &self,
        documents: &DashMap<Url, DocumentState>,
        position: Position,
    ) -> Vec<Location> {
        // TODO: Implement reference finding across all documents
        Vec::new()
    }

    pub fn document_symbols(&self) -> Vec<DocumentSymbol> {
        let mut symbols = Vec::new();

        if let Some(tree) = &self.tree {
            self.collect_symbols(tree.root_node(), &mut symbols);
        }

        symbols
    }

    fn collect_symbols(&self, node: tree_sitter::Node, symbols: &mut Vec<DocumentSymbol>) {
        if node.kind() == "binding" {
            if let Some(name_node) = node.child_by_field_name("name") {
                let start_pos = self.ts_position_to_lsp(node.start_position());
                let end_pos = self.ts_position_to_lsp(node.end_position());

                symbols.push(DocumentSymbol {
                    name: name_node.utf8_text(&self.source.as_bytes())
                        .unwrap_or("unknown").to_string(),
                    detail: None,
                    kind: SymbolKind::VARIABLE,
                    range: Range::new(start_pos, end_pos),
                    selection_range: Range::new(start_pos, end_pos),
                    children: None,
                    tags: None,
                    deprecated: None,
                });
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_symbols(child, symbols);
        }
    }

    fn ts_position_to_lsp(&self, pos: tree_sitter::Point) -> Position {
        Position::new(pos.row as u32, pos.column as u32)
    }
}
```

---

## 4. Performance: Implement Analysis Caching

### Current Issue
Hover and completion handlers re-parse and re-analyze entire document each time.

### Solution: Add AST Cache

```rust
// In document.rs
#[derive(Debug)]
pub struct DocumentState {
    pub source: String,
    pub tree: Option<tree_sitter::Tree>,
    pub diagnostics: Vec<Diagnostic>,
    pub type_checked: bool,

    // NEW: Cached typed expression
    cached_typed_expr: Option<melbi_core::types::TypedExpr>,
    cached_source_hash: u64,
}

impl DocumentState {
    pub fn new(source: String) -> Self {
        Self {
            source,
            tree: None,
            diagnostics: Vec::new(),
            type_checked: false,
            cached_typed_expr: None,
            cached_source_hash: 0,
        }
    }

    pub fn get_typed_expr(&mut self) -> Option<&melbi_core::types::TypedExpr> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.source.hash(&mut hasher);
        let current_hash = hasher.finish();

        // Return cached if source unchanged
        if current_hash == self.cached_source_hash {
            return self.cached_typed_expr.as_ref();
        }

        // Re-compute and cache
        let arena = Bump::new();
        let parsed = melbi_core::parser::parse(&arena, &self.source).ok()?;
        let type_manager = melbi_core::types::manager::TypeManager::new(&arena);

        match melbi_core::analyzer::analyze(
            type_manager,
            &arena,
            parsed,
            &[],
            &[],
        ) {
            Ok(typed_expr) => {
                self.cached_typed_expr = Some(typed_expr);
                self.cached_source_hash = current_hash;
                self.cached_typed_expr.as_ref()
            }
            Err(_) => None,
        }
    }

    pub fn hover_at_position(&mut self, position: Position) -> Option<String> {
        let typed_expr = self.get_typed_expr()?;

        // TODO: Implement span-based lookup instead of returning full expression
        Some(format!("```melbi\n{}\n```", typed_expr.expr.0))
    }
}
```

---

## 5. Completion: Implement Scope-Based Suggestions

### Current Issue
Completion returns empty list regardless of context.

### Solution

```rust
impl DocumentState {
    pub fn completions_at_position(&self, position: Position) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Collect identifiers in scope
        if let Some(tree) = &self.tree {
            self.collect_completions(
                tree.root_node(),
                position,
                &mut items,
            );
        }

        // Add keyword completions
        items.extend(vec![
            CompletionItem {
                label: "if".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("if condition then value else value".to_string()),
                insert_text: Some("if ${1:condition} then ${2:value} else ${3:value}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "where".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("where { ... }".to_string()),
                insert_text: Some("where {${1:binding} = ${2:value}}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "Record".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Record { ... }".to_string()),
                insert_text: Some("Record { ${1:field} = ${2:value} }".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
        ]);

        items
    }

    fn collect_completions(
        &self,
        node: tree_sitter::Node,
        position: Position,
        items: &mut Vec<CompletionItem>,
    ) {
        let node_pos = self.ts_position_to_lsp(node.start_position());

        // Only collect bindings before cursor position
        if node_pos.line > position.line
            || (node_pos.line == position.line && node_pos.character > position.character)
        {
            return;
        }

        if node.kind() == "binding" {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(self.source.as_bytes()) {
                    items.push(CompletionItem {
                        label: name.to_string(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        ..Default::default()
                    });
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_completions(child, position, items);
        }
    }

    fn ts_position_to_lsp(&self, pos: tree_sitter::Point) -> Position {
        Position::new(pos.row as u32, pos.column as u32)
    }
}
```

---

## 6. Add Snippets Support

### File Structure
Create `/home/user/melbi/vscode/snippets/melbi.json`:

```json
{
  "If Expression": {
    "prefix": "if",
    "body": "if ${1:condition} then\n  ${2:value_true}\nelse\n  ${3:value_false}",
    "description": "If-then-else expression"
  },
  "Where Binding": {
    "prefix": "where",
    "body": "${1:expr} where {\n  ${2:binding}} = ${3:value}\n}",
    "description": "Where binding expression"
  },
  "Record": {
    "prefix": "record",
    "body": "Record {\n  ${1:field} = ${2:value}\n}",
    "description": "Record constructor"
  },
  "Lambda": {
    "prefix": "lambda",
    "body": "\\(${1:args}) => ${2:body}",
    "description": "Lambda function"
  },
  "Format String": {
    "prefix": "fstring",
    "body": "f\"${1:text} {${2:expr}} ${3:more}\"",
    "description": "Format string with interpolation"
  },
  "Pattern Match": {
    "prefix": "match",
    "body": "match ${1:expr} {\n  ${2:pattern} => ${3:result}\n}",
    "description": "Pattern matching expression"
  }
}
```

Update `package.json`:

```json
{
  "contributes": {
    "snippets": [
      {
        "language": "melbi",
        "path": "./snippets/melbi.json"
      }
    ]
  }
}
```

---

## 7. Add Keybindings

Create `/home/user/melbi/vscode/keybindings/default.json`:

```json
[
  {
    "command": "editor.action.formatDocument",
    "key": "shift+alt+f",
    "when": "editorLangId == melbi"
  },
  {
    "command": "editor.action.goToDeclaration",
    "key": "ctrl+shift+d",
    "when": "editorLangId == melbi"
  },
  {
    "command": "editor.action.referenceSearch.trigger",
    "key": "shift+f12",
    "when": "editorLangId == melbi"
  },
  {
    "command": "editor.action.showHover",
    "key": "ctrl+k ctrl+i",
    "when": "editorLangId == melbi && editorTextFocus"
  }
]
```

Update `package.json`:

```json
{
  "contributes": {
    "keybindings": [
      {
        "command": "editor.action.formatDocument",
        "key": "shift+alt+f",
        "when": "editorLangId == melbi"
      }
    ]
  }
}
```

---

## 8. Create README Files

### `/home/user/melbi/zed/README.md`

```markdown
# Melbi Language Support for Zed

Provides IDE support for the Melbi expression language in Zed editor.

## Features

- Syntax highlighting with tree-sitter
- Intelligent bracket matching and auto-closing
- Code folding
- Document outline/symbols
- Type checking with diagnostics
- Code formatting (via LSP)
- Hover information showing types
- Automatic indentation

## Requirements

- Zed 0.x
- `melbi-lsp` language server (automatically located)

## Installation

This extension is included with Melbi. The language server (`melbi-lsp`) must be built and available in your PATH.

## Building from Source

```bash
cd /path/to/melbi
cargo build --release -p melbi-lsp
```

The binary will be available at `./target/release/melbi-lsp`.

## Features in Detail

### Syntax Highlighting

Uses tree-sitter for accurate syntax highlighting with support for:
- Keywords (if, then, else, where, as, otherwise)
- Format strings with interpolation
- Byte strings
- Comments

### Code Formatting

Format your code with the language server:
- Format document: `Ctrl+Shift+P` > "Format Document"
- Format on save: Configure in Zed settings

### Type Information

Hover over expressions to see their type:
```melbi
x = 42  -- hover over '42' to see the type
```

## Configuration

Edit `.zed/settings.json`:

```json
{
  "[melbi]": {
    "format_on_save": "on"
  }
}
```

## Troubleshooting

**Language server not starting:**
- Ensure `melbi-lsp` is in your PATH
- Check Zed logs: `View > Open Logs`

**Syntax highlighting not working:**
- Verify tree-sitter grammar is loaded
- Check file extension (.mb, .melbi, or .🖖)

## Contributing

Found a bug? Have a feature request?
[Open an issue](https://github.com/NiltonVolpato/melbi/issues)

## License

MIT OR Apache-2.0
```

### `/home/user/melbi/vscode/README.md`

```markdown
# Melbi Language Support for VS Code

Provides IDE support for the Melbi expression language in Visual Studio Code.

## Features

- Syntax highlighting with TextMate grammar
- Intelligent bracket matching and auto-closing
- Code folding
- Type checking with diagnostics
- Code formatting (via LSP)
- Hover information showing types
- Automatic indentation
- Code snippets

## Requirements

- VS Code 1.100.0 or higher
- `melbi-lsp` language server

## Installation

### From VS Code Marketplace

Search for "Melbi Language Support" in the Extensions marketplace.

### Manual Installation

```bash
git clone https://github.com/NiltonVolpato/melbi.git
cd melbi/vscode
npm install
npm run compile
# Package the extension
vsce package
# Install
code --install-extension melbi-0.1.0.vsix
```

## Building the Language Server

The extension requires the `melbi-lsp` binary. Build it:

```bash
cd /path/to/melbi
cargo build --release -p melbi-lsp
```

The extension will look for the binary at:
- **Development:** `../target/debug/melbi-lsp`
- **Production:** `./bin/melbi-lsp` (relative to extension)

## Usage

### File Support

The extension recognizes Melbi files with extensions:
- `.mb`
- `.melbi`
- `.🖖` (yes, Unicode file extensions are supported!)

### Snippets

Quickly insert common patterns:

| Snippet | Trigger | Example |
|---------|---------|---------|
| If expression | `if` | `if condition then value else value` |
| Where binding | `where` | `expr where { binding = value }` |
| Record | `record` | `Record { field = value }` |
| Lambda | `lambda` | `\(args) => body` |
| Format string | `fstring` | `f"text { expr } more"` |

### Commands

- **Format Document:** `Shift+Alt+F`
- **Show Hover:** `Ctrl+K Ctrl+I`
- **Go to Definition:** `Ctrl+Shift+D` (when implemented)

## Settings

Configure in your VS Code `settings.json`:

```json
{
  "[melbi]": {
    "editor.formatOnSave": true,
    "editor.formatOnPaste": true,
    "editor.tabSize": 4,
    "editor.insertSpaces": true
  }
}
```

## Troubleshooting

**Language server fails to start:**
- Check that `melbi-lsp` binary is accessible
- Open VS Code output channel: `View > Output`
- Look for "Melbi Language Server" output

**Syntax highlighting incorrect:**
- Try reloading: `Ctrl+Shift+P` > "Reload Window"
- Check file extension

**Formatting doesn't work:**
- Verify `melbi-lsp` is built with the formatter
- Check LSP logs in output

## Contributing

Issues and PRs welcome!
[GitHub Repository](https://github.com/NiltonVolpato/melbi)

## License

MIT OR Apache-2.0
```

---

## 9. Create Unit Tests for Extensions

### `/home/user/melbi/vscode/tests/extension.test.ts`

```typescript
import * as assert from 'assert';
import * as vscode from 'vscode';

suite('Melbi Extension', () => {
  test('should activate on Melbi file', async () => {
    const uri = vscode.Uri.file('/tmp/test.mb');
    const doc = await vscode.workspace.openTextDocument(uri);
    const editor = await vscode.window.showTextDocument(doc);

    const extension = vscode.extensions.getExtension('melbi.melbi');
    assert.ok(extension, 'Extension should be registered');
  });

  test('should recognize .melbi files', async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: 'melbi',
      content: 'x = 42',
    });

    assert.strictEqual(doc.languageId, 'melbi');
  });

  test('should have language configuration', async () => {
    const config = vscode.workspace.getConfiguration('melbi');
    assert.ok(config);
  });
});
```

---

## 10. Document Architecture

Create `/home/user/melbi/EXTENSION_ARCHITECTURE.md`:

```markdown
# Melbi IDE Extensions - Architecture

## Overview

```
┌──────────────────────────────────────────┐
│        VS Code / Zed Editor              │
└──────────────────┬───────────────────────┘
                   │
           ┌───────▼────────┐
           │  LSP Client    │ (vscode-languageclient / Zed built-in)
           └───────┬────────┘
                   │
    ───────────────┼─────────────────
    │              │              │
┌───▼──┐      ┌────▼─────┐   ┌──▼────┐
│Syntax│  LSP │ melbi-lsp │   │Format │
│      │◄──────│           │   │       │
└──────┘   │   └───┬───────┘   └──────┘
           │       │
           │   ┌───▼──────────────┐
           │   │  melbi-core      │
           │   │  melbi-fmt       │
           │   │  tree-sitter     │
           │   └──────────────────┘
           │
    ┌──────▼──────┐
    │tree-sitter  │
    │grammar      │
    │(external)   │
    └─────────────┘
```

### Data Flow

1. **File Opens:** Editor detects `.mb` file
2. **LSP Activate:** LSP client spawns `melbi-lsp` process
3. **Analysis:** melbi-lsp uses melbi-core for type checking
4. **Diagnostics:** Errors published back to editor
5. **Hover:** Editor requests type info, LSP responds
6. **Formatting:** Editor calls format handler, melbi-fmt processes code

### Extension Points

- **Syntax Highlighting:** Zed (tree-sitter), VS Code (TextMate)
- **Code Formatting:** Both → melbi-lsp → melbi-fmt
- **Type Information:** Both → melbi-lsp → melbi-core
- **Diagnostics:** Both ← melbi-lsp ← melbi-core + tree-sitter
```

---

## Implementation Priority

### Week 1 (Critical Fixes)
- [ ] Fix Zed grammar path hardcoding
- [ ] Add VS Code activation events
- [ ] Create README files

### Week 2 (LSP Features)
- [ ] Implement definition_provider
- [ ] Implement references_provider
- [ ] Add analysis caching

### Week 3 (UX Features)
- [ ] Add snippets support
- [ ] Implement scope-based completions
- [ ] Add keybindings

### Week 4+ (Polish)
- [ ] Document symbols in both editors
- [ ] Code actions
- [ ] Theme customization
