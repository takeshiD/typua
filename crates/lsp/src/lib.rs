use std::fs::File;
use std::sync::Arc;

use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::info;
use tracing_subscriber::EnvFilter;

use typua_ty::error::TypuaError;

#[derive(Debug)]
struct Backend {
    client: Client,
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
}

async fn run_lsp_service() {
    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}

pub fn handle_lsp_service() {
    let log_path = xdg::BaseDirectories::with_prefix("typua")
        .place_cache_file("log.jsonl")
        .expect("failed to create log dir");
    let log_file = if !log_path.exists() {
        Arc::new(File::create(log_path).expect("failed to create log file"))
    } else {
        Arc::new(
            File::options()
                .append(true)
                .open(log_path)
                .expect("failed to open log file"),
        )
    };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(false)
        .with_writer(log_file)
        .json()
        .init();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|source| TypuaError::Runtime { source })
        .expect("failed runtime to start");
    runtime.block_on(run_lsp_service())
}
