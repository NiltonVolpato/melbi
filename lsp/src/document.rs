use bumpalo::Bump;
use tower_lsp::lsp_types::*;

/// Represents the state of a document being edited
#[derive(Debug)]
pub struct DocumentState {
    /// The source code
    pub source: String,

    /// Tree-sitter parse tree (if parsing succeeded)
    pub tree: Option<tree_sitter::Tree>,

    /// Current diagnostics for this document
    pub diagnostics: Vec<Diagnostic>,

    /// Whether the document type-checked successfully
    pub type_checked: bool,
}

impl DocumentState {
    pub fn new(source: String) -> Self {
        Self {
            source,
            tree: None,
            diagnostics: Vec::new(),
            type_checked: false,
        }
    }

    /// Update the document with new source code
    pub fn update(&mut self, source: String) {
        self.source = source;
        self.tree = None;
        self.diagnostics.clear();
        self.type_checked = false;
    }

    /// Parse and analyze the document, returning all diagnostics
    pub fn analyze(&mut self) -> Vec<Diagnostic> {
        let mut all_diagnostics = Vec::new();

        // Parse with tree-sitter for syntax errors
        let syntax_diagnostics = self.parse_with_tree_sitter();
        all_diagnostics.extend(syntax_diagnostics);

        // If tree-sitter parsing succeeded, try semantic analysis
        if self.tree.is_some() {
            let type_diagnostics = self.type_check();
            all_diagnostics.extend(type_diagnostics);
        }

        self.diagnostics = all_diagnostics.clone();
        all_diagnostics
    }

    /// Parse the document using tree-sitter
    fn parse_with_tree_sitter(&mut self) -> Vec<Diagnostic> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_melbi::LANGUAGE.into())
            .expect("Error loading Melbi grammar");

        let tree = match parser.parse(&self.source, None) {
            Some(tree) => tree,
            None => {
                return vec![Diagnostic {
                    range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    source: Some("melbi".to_string()),
                    message: "Failed to parse document".to_string(),
                    ..Default::default()
                }];
            }
        };

        let mut diagnostics = Vec::new();

        // Check for syntax errors
        if tree.root_node().has_error() {
            self.collect_syntax_errors(tree.root_node(), &mut diagnostics);
        }

        self.tree = Some(tree);
        diagnostics
    }

    /// Recursively collect syntax errors from tree-sitter parse tree
    fn collect_syntax_errors(&self, node: tree_sitter::Node, diagnostics: &mut Vec<Diagnostic>) {
        if node.is_error() || node.is_missing() {
            let start = node.start_position();
            let end = node.end_position();

            diagnostics.push(Diagnostic {
                range: Range::new(
                    Position::new(start.row as u32, start.column as u32),
                    Position::new(end.row as u32, end.column as u32),
                ),
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                source: Some("melbi".to_string()),
                message: if node.is_missing() {
                    format!("Missing {}", node.kind())
                } else {
                    "Syntax error".to_string()
                },
                ..Default::default()
            });
        }

        // Recursively check children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_syntax_errors(child, diagnostics);
        }
    }

    /// Analyze the document for type errors
    fn type_check(&mut self) -> Vec<Diagnostic> {
        use melbi_core::{analyzer, parser, types::manager::TypeManager};

        // Create arena for this analysis
        let arena = Bump::new();

        // Parse with Pest
        let parsed = match parser::parse(&arena, &self.source) {
            Ok(p) => p,
            Err(_) => {
                // Parsing failed - tree-sitter already reported errors
                return Vec::new();
            }
        };

        // Create type manager
        let type_manager = TypeManager::new(&arena);

        // Analyze with empty globals and variables for now
        // TODO: Add support for providing globals (stdlib functions)
        let globals: &[(&str, &_)] = &[];
        let variables: &[(&str, &_)] = &[];

        match analyzer::analyze(type_manager, &arena, parsed, globals, variables) {
            Ok(_typed_expr) => {
                self.type_checked = true;
                Vec::new()
            }
            Err(e) => {
                self.type_checked = false;
                vec![self.error_to_diagnostic(&e)]
            }
        }
    }

    /// Convert a Melbi error to an LSP diagnostic
    fn error_to_diagnostic(&self, error: &melbi_core::errors::Error) -> Diagnostic {
        use melbi_core::errors::ErrorKind;

        let (message, range, severity) = match error.kind.as_ref() {
            ErrorKind::TypeChecking { help, span, .. } => {
                let range = span.as_ref().map(|s| {
                    let start_pos = self.offset_to_position(s.0.start);
                    let end_pos = self.offset_to_position(s.0.end);
                    Range::new(start_pos, end_pos)
                }).unwrap_or_else(|| Range::new(Position::new(0, 0), Position::new(0, 0)));

                let message = help.clone().unwrap_or_else(|| "Type error".to_string());
                (message, range, DiagnosticSeverity::ERROR)
            }
            ErrorKind::Parse { help, err_span, .. } => {
                let start_pos = self.offset_to_position(err_span.0.start);
                let end_pos = self.offset_to_position(err_span.0.end);
                let range = Range::new(start_pos, end_pos);
                let message = help.clone().unwrap_or_else(|| "Parse error".to_string());
                (message, range, DiagnosticSeverity::ERROR)
            }
            ErrorKind::TypeConversion { help, span, .. } => {
                let start_pos = self.offset_to_position(span.0.start);
                let end_pos = self.offset_to_position(span.0.end);
                let range = Range::new(start_pos, end_pos);
                (help.clone(), range, DiagnosticSeverity::ERROR)
            }
            ErrorKind::MapsNotYetImplemented { span, .. } => {
                let start_pos = self.offset_to_position(span.0.start);
                let end_pos = self.offset_to_position(span.0.end);
                let range = Range::new(start_pos, end_pos);
                ("Maps not yet implemented".to_string(), range, DiagnosticSeverity::ERROR)
            }
            ErrorKind::Whatever { message, .. } => {
                (message.clone(), Range::new(Position::new(0, 0), Position::new(0, 0)), DiagnosticSeverity::ERROR)
            }
        };

        Diagnostic {
            range,
            severity: Some(severity),
            code: None,
            source: Some("melbi".to_string()),
            message,
            ..Default::default()
        }
    }

    /// Convert byte offset to LSP Position
    fn offset_to_position(&self, offset: usize) -> Position {
        let mut line = 0;
        let mut col = 0;
        let mut current_offset = 0;

        for ch in self.source.chars() {
            if current_offset >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
            current_offset += ch.len_utf8();
        }

        Position::new(line, col)
    }

    /// Get hover information at a position
    pub fn hover_at_position(&self, _position: Position) -> Option<String> {
        use melbi_core::{analyzer, parser, types::manager::TypeManager};

        // Only provide hover if type checking succeeded
        if !self.type_checked {
            return None;
        }

        // Re-run analysis to get typed expression
        // TODO: Cache the typed expression to avoid re-analysis
        let arena = Bump::new();
        let parsed = parser::parse(&arena, &self.source).ok()?;
        let type_manager = TypeManager::new(&arena);
        let globals: &[(&str, &_)] = &[];
        let variables: &[(&str, &_)] = &[];

        let typed_expr = analyzer::analyze(type_manager, &arena, parsed, globals, variables).ok()?;

        // For now, just return the type of the entire expression
        // TODO: Implement proper span-based lookup in the typed AST to find the exact
        //       expression at the cursor position
        // TODO: Add documentation from comments when available (see DOCUMENTATION_COMMENTS.md)

        // Format the hover response
        let type_str = format!("{}", typed_expr.expr.0);
        let hover_text = format!("```melbi\n{}\n```", type_str);

        // TODO: When documentation support is added, append it here:
        // if let Some(doc) = get_documentation_for_expr(typed_expr.expr) {
        //     hover_text.push_str("\n\n---\n\n");
        //     hover_text.push_str(doc);
        // }

        Some(hover_text)
    }

    /// Get completion items at a position
    pub fn completions_at_position(&self, _position: Position) -> Vec<CompletionItem> {
        // TODO: Implement proper completion based on scope
        // For now, return empty list
        Vec::new()
    }
}
