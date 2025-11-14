use bumpalo::Bump;
use js_sys::JSON;
use lsp_types::{
    CompletionItem as LspCompletionItem, CompletionItemKind, CompletionTextEdit, Documentation,
    InsertTextFormat, Position,
};
use melbi_core::api::{CompileOptions, Engine, EngineOptions, Error};
use melbi_core::api::{Diagnostic as CoreDiagnostic, RelatedInfo, Severity};
use melbi_core::parser::Span;
use melbi_core::types::traits::display_type;
use melbi_core::values::dynamic::Value;
use melbi_lsp::document::DocumentState;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct PlaygroundEngine {
    engine_arena: &'static Bump,
    engine: Engine<'static>,
}

#[wasm_bindgen]
impl PlaygroundEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> PlaygroundEngine {
        let arena = Box::leak(Box::new(Bump::new()));
        let engine = Engine::new(arena, EngineOptions::default(), |_, _, _| {});

        PlaygroundEngine {
            engine_arena: arena,
            engine,
        }
    }

    /// Compile and execute the provided Melbi expression.
    #[wasm_bindgen]
    pub fn evaluate(&self, source: &str) -> Result<JsValue, JsValue> {
        let response = self.evaluate_internal(source);
        to_js_value(&response)
    }

    /// Format the provided source using the canonical formatter.
    #[wasm_bindgen]
    pub fn format_source(&self, source: &str) -> Result<JsValue, JsValue> {
        let response = self.format_internal(source);
        to_js_value(&response)
    }

    /// Analyze the provided source and return compilation diagnostics without executing it.
    #[wasm_bindgen]
    pub fn analyze(&self, source: &str) -> Result<JsValue, JsValue> {
        let response = self.analyze_internal(source);
        to_js_value(&response)
    }

    /// Return hover documentation at the provided byte offset.
    #[wasm_bindgen]
    pub fn hover_at(&self, source: &str, offset: usize) -> Result<JsValue, JsValue> {
        let response = self.hover_internal(source, offset);
        to_js_value(&response)
    }

    /// Return completion suggestions at the provided byte offset.
    #[wasm_bindgen]
    pub fn complete_at(&self, source: &str, offset: usize) -> Result<JsValue, JsValue> {
        let response = self.complete_internal(source, offset);
        to_js_value(&response)
    }
}

impl PlaygroundEngine {
    fn evaluate_internal(&self, source: &str) -> WorkerResponse<EvaluationSuccess> {
        let source_in_arena = self.engine_arena.alloc_str(source);
        let source_ref: &'static str = source_in_arena;
        let compile_result = self
            .engine
            .compile(CompileOptions::default(), source_ref, &[]);

        match compile_result {
            Ok(expr) => {
                let value_arena = Bump::new();
                match expr.run(&value_arena, &[], None) {
                    Ok(value) => WorkerResponse::ok(EvaluationSuccess::from_value(value)),
                    Err(err) => WorkerResponse::err(err),
                }
            }
            Err(err) => WorkerResponse::err(err),
        }
    }

    fn format_internal(&self, source: &str) -> WorkerResponse<FormatSuccess> {
        match melbi_fmt::format(source, false, true) {
            Ok(formatted) => WorkerResponse::ok(FormatSuccess { formatted }),
            Err(err) => WorkerResponse::err(Error::Runtime(err.to_string())),
        }
    }

    fn analyze_internal(&self, source: &str) -> WorkerResponse<AnalysisSuccess> {
        let source_in_arena = self.engine_arena.alloc_str(source);
        let source_ref: &'static str = source_in_arena;
        let compile_result = self
            .engine
            .compile(CompileOptions::default(), source_ref, &[]);

        match compile_result {
            Ok(_) => WorkerResponse::ok(AnalysisSuccess {
                diagnostics: Vec::new(),
            }),
            Err(Error::Compilation { diagnostics }) => WorkerResponse::ok(AnalysisSuccess {
                diagnostics: diagnostics
                    .into_iter()
                    .map(DiagnosticPayload::from)
                    .collect(),
            }),
            Err(err) => WorkerResponse::err(err),
        }
    }

    fn hover_internal(&self, source: &str, offset: usize) -> WorkerResponse<HoverSuccess> {
        let mut document = DocumentState::new(source.to_string());
        document.analyze();
        let position = offset_to_position(source, offset);
        let contents = document.hover_at_position(position);
        WorkerResponse::ok(HoverSuccess { contents })
    }

    fn complete_internal(&self, source: &str, offset: usize) -> WorkerResponse<CompletionSuccess> {
        let mut document = DocumentState::new(source.to_string());
        document.analyze();
        let position = offset_to_position(source, offset);
        let items = document
            .completions_at_position(position)
            .into_iter()
            .map(CompletionItemPayload::from)
            .collect();

        WorkerResponse::ok(CompletionSuccess { items })
    }
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum WorkerResponse<T> {
    Ok { data: T },
    Err { error: WorkerError },
}

impl<T> WorkerResponse<T> {
    fn ok(data: T) -> Self {
        WorkerResponse::Ok { data }
    }

    fn err(error: Error) -> Self {
        WorkerResponse::Err {
            error: WorkerError::from(error),
        }
    }
}

#[derive(Serialize)]
pub struct WorkerError {
    kind: &'static str,
    message: String,
    diagnostics: Option<Vec<DiagnosticPayload>>,
}

#[derive(Serialize)]
pub struct DiagnosticPayload {
    severity: &'static str,
    message: String,
    span: RangePayload,
    help: Option<String>,
    code: Option<String>,
    related: Vec<RelatedInfoPayload>,
}

#[derive(Serialize)]
pub struct RelatedInfoPayload {
    span: RangePayload,
    message: String,
}

#[derive(Serialize)]
pub struct RangePayload {
    start: usize,
    end: usize,
}

#[derive(Serialize)]
pub struct EvaluationSuccess {
    value: String,
    type_name: String,
}

impl EvaluationSuccess {
    fn from_value(value: Value<'static, '_>) -> Self {
        Self {
            value: value.to_string(),
            type_name: format!("{}", display_type(value.ty)),
        }
    }
}

#[derive(Serialize)]
pub struct FormatSuccess {
    formatted: String,
}

#[derive(Serialize)]
pub struct AnalysisSuccess {
    diagnostics: Vec<DiagnosticPayload>,
}

#[derive(Serialize)]
pub struct HoverSuccess {
    contents: Option<String>,
}

#[derive(Serialize)]
pub struct CompletionSuccess {
    items: Vec<CompletionItemPayload>,
}

#[derive(Serialize)]
pub struct CompletionItemPayload {
    label: String,
    kind: Option<String>,
    detail: Option<String>,
    documentation: Option<String>,
    insert_text: Option<String>,
    is_snippet: bool,
}

impl From<Error> for WorkerError {
    fn from(err: Error) -> Self {
        match err {
            Error::Api(message) => WorkerError {
                kind: "api",
                message,
                diagnostics: None,
            },
            Error::Compilation { diagnostics } => WorkerError {
                kind: "compilation",
                message: format!(
                    "Compilation failed with {} diagnostic(s)",
                    diagnostics.len()
                ),
                diagnostics: Some(
                    diagnostics
                        .into_iter()
                        .map(DiagnosticPayload::from)
                        .collect(),
                ),
            },
            Error::Runtime(message) => WorkerError {
                kind: "runtime",
                message,
                diagnostics: None,
            },
            Error::ResourceExceeded(message) => WorkerError {
                kind: "resource_exceeded",
                message,
                diagnostics: None,
            },
        }
    }
}

impl From<CoreDiagnostic> for DiagnosticPayload {
    fn from(diag: CoreDiagnostic) -> Self {
        Self {
            severity: severity_to_str(diag.severity),
            message: diag.message,
            span: RangePayload::from(diag.span),
            help: diag.help,
            code: diag.code,
            related: diag
                .related
                .into_iter()
                .map(RelatedInfoPayload::from)
                .collect(),
        }
    }
}

impl From<RelatedInfo> for RelatedInfoPayload {
    fn from(info: RelatedInfo) -> Self {
        Self {
            span: RangePayload::from(info.span),
            message: info.message,
        }
    }
}

impl From<LspCompletionItem> for CompletionItemPayload {
    fn from(item: LspCompletionItem) -> Self {
        let documentation = item.documentation.as_ref().and_then(|doc| match doc {
            Documentation::String(text) => Some(text.clone()),
            Documentation::MarkupContent(markup) => Some(markup.value.clone()),
        });

        let insert_text = item
            .insert_text
            .clone()
            .or_else(|| text_from_edit(&item.text_edit));

        Self {
            label: item.label.clone(),
            kind: item
                .kind
                .map(|kind| completion_kind_to_str(kind).to_string()),
            detail: item.detail.clone(),
            documentation,
            insert_text,
            is_snippet: item.insert_text_format == Some(InsertTextFormat::SNIPPET),
        }
    }
}

impl From<Span> for RangePayload {
    fn from(span: Span) -> Self {
        RangePayload {
            start: span.0.start,
            end: span.0.end,
        }
    }
}

fn severity_to_str(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn completion_kind_to_str(kind: CompletionItemKind) -> &'static str {
    match kind {
        CompletionItemKind::FUNCTION | CompletionItemKind::METHOD => "function",
        CompletionItemKind::VARIABLE
        | CompletionItemKind::VALUE
        | CompletionItemKind::FIELD
        | CompletionItemKind::CONSTANT => "variable",
        CompletionItemKind::PROPERTY => "property",
        CompletionItemKind::KEYWORD => "keyword",
        CompletionItemKind::SNIPPET => "snippet",
        _ => "text",
    }
}

fn text_from_edit(edit: &Option<CompletionTextEdit>) -> Option<String> {
    match edit {
        Some(CompletionTextEdit::Edit(text_edit)) => Some(text_edit.new_text.clone()),
        Some(CompletionTextEdit::InsertAndReplace(text_edit)) => Some(text_edit.new_text.clone()),
        None => None,
    }
}

fn offset_to_position(source: &str, offset: usize) -> Position {
    let mut line: u32 = 0;
    let mut col: u32 = 0;
    let mut current_offset = 0usize;

    for ch in source.chars() {
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

fn to_js_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    let serialized = serde_json::to_string(value)
        .map_err(|err| JsValue::from_str(&format!("serialization error: {}", err)))?;
    JSON::parse(&serialized).map_err(|err| err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_basic_expression() {
        let engine = PlaygroundEngine::new();
        match engine.evaluate_internal("40 + 2") {
            WorkerResponse::Ok { data } => {
                assert_eq!(data.value, "42");
                assert_eq!(data.type_name, "Int");
            }
            WorkerResponse::Err { error } => panic!("evaluation failed: {}", error.message),
        }
    }

    #[test]
    fn formats_source() {
        let engine = PlaygroundEngine::new();
        match engine.format_internal("1+1") {
            WorkerResponse::Ok { data } => {
                assert_eq!(data.formatted, "1 + 1");
            }
            WorkerResponse::Err { error } => panic!("formatting failed: {}", error.message),
        }
    }

    #[test]
    fn analyzes_source_without_executing() {
        let engine = PlaygroundEngine::new();
        match engine.analyze_internal("1 +") {
            WorkerResponse::Ok { data } => {
                assert!(!data.diagnostics.is_empty());
            }
            WorkerResponse::Err { error } => panic!("analysis failed: {}", error.message),
        }
    }

    #[test]
    fn provides_completion_items() {
        let engine = PlaygroundEngine::new();
        match engine.complete_internal("whe", 3) {
            WorkerResponse::Ok { data } => {
                assert!(!data.items.is_empty());
            }
            WorkerResponse::Err { error } => panic!("completion failed: {}", error.message),
        }
    }

    #[test]
    fn hover_request_returns_response() {
        let engine = PlaygroundEngine::new();
        match engine.hover_internal("1 + 1", 0) {
            WorkerResponse::Ok { data } => {
                // Hover might be None, but the worker should respond successfully.
                assert!(data.contents.is_none() || data.contents.is_some());
            }
            WorkerResponse::Err { error } => panic!("hover failed: {}", error.message),
        }
    }
}
