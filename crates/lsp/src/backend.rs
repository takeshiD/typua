use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{
    Diagnostic as LspDiagnostic, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, InitializedParams, MessageType, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind,
};
use tower_lsp::{Client, LanguageServer};
use tracing::{debug, info};
use typua_analyzer::Analyzer;
use typua_config::LuaVersion;

#[derive(Debug)]
pub struct Backend {
    pub client: Client,
    pub analyzer: Analyzer,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            analyzer: Analyzer::new(),
        }
    }
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
        let uri = params.text_document.uri;
        let content = params.text_document.text;
        let analyze_result = self
            .analyzer
            .analyze(uri.as_ref(), &content, &LuaVersion::Lua51);
        self.client
            .log_message(MessageType::INFO, format!("File open {}", uri))
            .await;
        let diag: Vec<LspDiagnostic> = analyze_result
            .diagnotics
            .iter()
            .map(|d| {
                let d: LspDiagnostic = d.clone().into();
                d
            })
            .collect();
        for d in diag.iter() {
            debug!(
                "(line:{}, col:{}) {}",
                d.range.start.line, d.range.start.character, d.message
            );
        }
        self.client.publish_diagnostics(uri, diag, None).await
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
