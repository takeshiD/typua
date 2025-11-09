use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use tracing::info;

#[derive(Debug)]
pub struct Backend {
    pub client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        info!("initialize");
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                ..ServerCapabilities::default()
            },
        })
    }
    async fn initialized(&self, _: InitializedParams) {
        info!("initialized");
        self.client
            .log_message(MessageType::INFO, "initialized")
            .await;
    }
    async fn shutdown(&self) -> LspResult<()> {
        info!("shutdown");
        Ok(())
    }
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        info!("did open: {}", params.text_document.uri);
        self.client
            .log_message(
                MessageType::INFO,
                format!("File open {}", params.text_document.uri),
            )
            .await;
    }
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        info!("did close: {}", params.text_document.uri);
        self.client
            .log_message(
                MessageType::INFO,
                format!("File close {}", params.text_document.uri),
            )
            .await;
    }
}
