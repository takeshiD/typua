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
    let log_name = "log.jsonl";
    let log_path = match xdg::BaseDirectories::with_prefix("typua").place_cache_file(log_name) {
        Ok(log_path) => {
            println!("Get log path: {}", log_path.display());
            log_path
        }
        Err(e) => {
            eprintln!("Failed get log path: {e}");
            return;
        }
    };
    let log_file = if !log_path.exists() {
        match File::create(&log_path) {
            Ok(log_file) => {
                println!("Create log file: {}", log_path.display());
                Arc::new(log_file)
            }
            Err(e) => {
                eprintln!("Failed to create log file: {e}");
                return;
            }
        }
    } else {
        match File::options().append(true).open(&log_path) {
            Ok(log_file) => {
                println!("Already exist log file: {}", log_path.display());
                Arc::new(log_file)
            }
            Err(e) => {
                eprintln!("failed to open log file: {e}");
                return;
            }
        }
    };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(false)
        .with_writer(log_file)
        .json()
        .init();
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|source| TypuaError::Runtime { source })
    {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("Failed to start runtime: {e}");
            return;
        }
    };
    runtime.block_on(run_lsp_service())
}
