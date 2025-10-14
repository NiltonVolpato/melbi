use melbi_core::parser::{ExpressionParser, Rule};
use pest::Parser;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::helpers::IntoDiagnostics;

mod helpers;

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                // Add more capabilities as you implement them
                ..Default::default()
            },
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

        let diagnostics = match ExpressionParser::parse(Rule::main, document.text.as_str()) {
            Ok(_) => vec![], // No diagnostics if parsing is successful
            Err(err) => vec![err].into_diagnostics(),
        };

        self.client
            .publish_diagnostics(document.uri, diagnostics, None)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File changed!")
            .await;
        let DidChangeTextDocumentParams {
            text_document,
            content_changes,
        } = params;
        let VersionedTextDocumentIdentifier { uri, .. } = text_document;

        assert_eq!(content_changes.len(), 1);
        let change = content_changes.into_iter().next().unwrap();
        assert!(change.range.is_none());

        let diagnostics = match ExpressionParser::parse(Rule::main, change.text.as_str()) {
            Ok(_) => vec![], // No diagnostics if parsing is successful
            Err(err) => vec![err].into_diagnostics(),
        };

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
