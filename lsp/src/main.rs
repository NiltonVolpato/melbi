use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

mod document;

use document::DocumentState;

#[derive(Debug)]
struct Backend {
    client: Client,
    /// Document cache, keyed by URI
    documents: DashMap<Url, DocumentState>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
        }
    }

    /// Analyze a document and publish diagnostics
    async fn analyze_document(&self, uri: Url) {
        // Analyze the document
        let all_diagnostics = {
            if let Some(mut doc) = self.documents.get_mut(&uri) {
                doc.analyze()
            } else {
                Vec::new()
            }
        }; // DashMap reference dropped here

        // Publish diagnostics
        self.client
            .publish_diagnostics(uri, all_diagnostics, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string()]),
                    ..Default::default()
                }),
                document_formatting_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::KEYWORD,
                                    SemanticTokenType::VARIABLE,
                                    SemanticTokenType::FUNCTION,
                                    SemanticTokenType::PARAMETER,
                                    SemanticTokenType::TYPE,
                                    SemanticTokenType::PROPERTY,
                                    SemanticTokenType::NUMBER,
                                    SemanticTokenType::STRING,
                                    SemanticTokenType::COMMENT,
                                    SemanticTokenType::OPERATOR,
                                ],
                                token_modifiers: vec![],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            ..Default::default()
                        },
                    ),
                ),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "Melbi Language Server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Melbi LSP initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File opened!")
            .await;

        let document = params.text_document;
        let uri = document.uri;
        let source = document.text;

        // Create document state
        let doc_state = DocumentState::new(source);
        self.documents.insert(uri.clone(), doc_state);

        // Analyze and publish diagnostics
        self.analyze_document(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File changed!")
            .await;

        let DidChangeTextDocumentParams {
            text_document,
            content_changes,
        } = params;
        let uri = text_document.uri;

        // We're using FULL sync, so there should be exactly one change
        if let Some(change) = content_changes.into_iter().next() {
            // Update document
            if let Some(mut doc) = self.documents.get_mut(&uri) {
                doc.update(change.text);
            }

            // Analyze and publish diagnostics
            self.analyze_document(uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        // Remove document from cache
        self.documents.remove(&params.text_document.uri);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let hover_text = {
            self.documents
                .get(&uri)
                .and_then(|doc| doc.hover_at_position(position))
        }; // DashMap reference dropped here

        Ok(hover_text.map(|text| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: text,
            }),
            range: None,
        }))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let items = {
            self.documents
                .get(&uri)
                .map(|doc| doc.completions_at_position(position))
                .unwrap_or_default()
        }; // DashMap reference dropped here

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;

        // Get source code, then drop the DashMap reference
        let source = {
            match self.documents.get(&uri) {
                Some(doc) => doc.source.clone(),
                None => return Ok(None),
            }
        }; // DashMap reference dropped here

        // Use the melbi-fmt formatter
        match melbi_fmt::format(&source, false, true) {
            Ok(formatted) => {
                // If the formatted text is the same, no edits needed
                if formatted == source {
                    return Ok(None);
                }

                // Calculate the range of the entire document
                let lines: Vec<&str> = source.lines().collect();
                let last_line = lines.len().saturating_sub(1) as u32;
                let last_char = lines.last().map(|l| l.len()).unwrap_or(0) as u32;

                let range = Range {
                    start: Position::new(0, 0),
                    end: Position::new(last_line, last_char),
                };

                Ok(Some(vec![TextEdit {
                    range,
                    new_text: formatted,
                }]))
            }
            Err(e) => {
                self.client
                    .log_message(MessageType::ERROR, format!("Format error: {}", e))
                    .await;
                Ok(None)
            }
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;

        let tokens = {
            self.documents
                .get(&uri)
                .and_then(|doc| doc.semantic_tokens())
        }; // DashMap reference dropped here

        Ok(tokens.map(|data| {
            SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data,
            })
        }))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
