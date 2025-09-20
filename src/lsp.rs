use std::{collections::HashMap, path::PathBuf, sync::Arc};

use full_moon::Error as FullMoonError;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, InitializeParams, InitializeResult,
    InitializedParams, MessageType, Position, Range, ServerCapabilities,
    TextDocumentContentChangeEvent, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, Url,
};
use tower_lsp::{Client, LanguageServer, LspService, Server, async_trait};

use crate::cli::LspOptions;
use crate::error::Result;

#[derive(Debug)]
pub struct TypuaLanguageServer {
    client: Client,
    _root: PathBuf,
    _config: Arc<crate::config::Config>,
    documents: RwLock<HashMap<Url, String>>,
}

impl TypuaLanguageServer {
    pub fn new(client: Client, options: LspOptions) -> Self {
        Self {
            client,
            _root: options.root,
            _config: Arc::new(options.config),
            documents: RwLock::new(HashMap::new()),
        }
    }

    async fn publish_diagnostics(&self, uri: Url, text: &str) {
        let diagnostics = match full_moon::parse(text) {
            Ok(_) => Vec::new(),
            Err(errors) => errors.into_iter().map(convert_error).collect(),
        };

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn update_document(&self, uri: Url, text: String) {
        {
            let mut documents = self.documents.write().await;
            documents.insert(uri.clone(), text.clone());
        }
        self.publish_diagnostics(uri, &text).await;
    }

    async fn remove_document(&self, uri: &Url) {
        {
            let mut documents = self.documents.write().await;
            documents.remove(uri);
        }
        self.client
            .publish_diagnostics(uri.clone(), Vec::new(), None)
            .await;
    }

    fn apply_change(text: &mut String, change: TextDocumentContentChangeEvent) {
        if change.range.is_none() {
            *text = change.text;
            return;
        }

        // TextDocumentSyncKind::FULL guarantees full content updates.
        *text = change.text;
    }
}

#[async_trait]
impl LanguageServer for TypuaLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        let text_document_sync = TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
            open_close: Some(true),
            change: Some(TextDocumentSyncKind::FULL),
            will_save: Some(false),
            will_save_wait_until: Some(false),
            save: None,
        });

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(text_document_sync),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Typua language server initialized")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: tower_lsp::lsp_types::DidOpenTextDocumentParams) {
        let text_document = params.text_document;
        self.update_document(text_document.uri, text_document.text)
            .await;
    }

    async fn did_change(&self, params: tower_lsp::lsp_types::DidChangeTextDocumentParams) {
        if params.content_changes.is_empty() {
            return;
        }

        let mut text = {
            let documents = self.documents.read().await;
            documents
                .get(&params.text_document.uri)
                .cloned()
                .unwrap_or_default()
        };

        for change in params.content_changes {
            Self::apply_change(&mut text, change);
        }

        self.update_document(params.text_document.uri, text).await;
    }

    async fn did_close(&self, params: tower_lsp::lsp_types::DidCloseTextDocumentParams) {
        self.remove_document(&params.text_document.uri).await;
    }
}

fn convert_error(error: FullMoonError) -> LspDiagnostic {
    let (start, end) = error.range();
    LspDiagnostic {
        range: Range {
            start: lsp_position(start),
            end: lsp_position(end),
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("typua".to_string()),
        message: error.error_message().to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}

fn lsp_position(position: full_moon::tokenizer::Position) -> Position {
    Position {
        line: position.line().saturating_sub(1) as u32,
        character: position.character().saturating_sub(1) as u32,
    }
}

pub async fn run(options: LspOptions) -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let shared = Arc::new(options);

    let (service, socket) = LspService::new(move |client| {
        let options = shared.as_ref().clone();
        TypuaLanguageServer::new(client, options)
    });

    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}
