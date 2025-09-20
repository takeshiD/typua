use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use full_moon::Error as FullMoonError;
use tokio::sync::RwLock;
use tower_lsp::{jsonrpc::Result as LspResult, lsp_types::HoverProviderCapability};
use tower_lsp::lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticOptions, DiagnosticServerCapabilities,
    DiagnosticSeverity, Hover, HoverContents, HoverParams, InitializeParams, InitializeResult,
    InitializedParams, MarkupContent, MarkupKind, MessageType, Position, Range, ServerCapabilities,
    TextDocumentContentChangeEvent, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, Url, WorkDoneProgressOptions,
};
use tower_lsp::{Client, LanguageServer, LspService, Server, async_trait};

use crate::checker;
use crate::cli::LspOptions;
use crate::diagnostics::{Diagnostic as CheckerDiagnostic, Severity, TextRange};
use crate::error::Result;

#[derive(Debug)]
pub struct TypuaLanguageServer {
    client: Client,
    _root: PathBuf,
    _config: Arc<crate::config::Config>,
    documents: RwLock<HashMap<Url, DocumentState>>,
}

#[derive(Debug, Clone)]
struct DocumentState {
    text: String,
    types: HashMap<(usize, usize), String>,
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

    async fn update_document(&self, uri: Url, text: String) {
        let (diagnostics, types) = self.analyze_document(&uri, &text);

        {
            let mut documents = self.documents.write().await;
            documents.insert(
                uri.clone(),
                DocumentState {
                    text: text.clone(),
                    types,
                },
            );
        }

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
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

    fn analyze_document(
        &self,
        uri: &Url,
        text: &str,
    ) -> (Vec<LspDiagnostic>, HashMap<(usize, usize), String>) {
        match full_moon::parse(text) {
            Ok(ast) => {
                let path = uri_to_path(uri);
                let result = checker::check_ast(&path, text, &ast);
                let diagnostics = result
                    .diagnostics
                    .into_iter()
                    .map(convert_checker_diagnostic)
                    .collect();
                (diagnostics, result.type_map)
            }
            Err(errors) => (
                errors.into_iter().map(convert_error).collect(),
                HashMap::new(),
            ),
        }
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
                hover_provider: Some(HoverProviderCapability::Simple(true)),
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
                .map(|doc| doc.text.clone())
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

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let documents = self.documents.read().await;
        if let Some(state) = documents.get(&uri) {
            let line = position.line as usize + 1;
            let character = position.character as usize + 1;
            if let Some(ty) = state.types.get(&(line, character)) {
                let contents = HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::PlainText,
                    value: format!("type: {ty}"),
                });
                return Ok(Some(Hover {
                    contents,
                    range: None,
                }));
            }
        }

        Ok(None)
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

fn convert_checker_diagnostic(diagnostic: CheckerDiagnostic) -> LspDiagnostic {
    let severity = match diagnostic.severity {
        Severity::Error => Some(DiagnosticSeverity::ERROR),
        Severity::Warning => Some(DiagnosticSeverity::WARNING),
        Severity::Information => Some(DiagnosticSeverity::INFORMATION),
        Severity::Hint => Some(DiagnosticSeverity::HINT),
    };

    let range = diagnostic
        .range
        .map(lsp_range_from_text)
        .unwrap_or_else(default_range);

    LspDiagnostic {
        range,
        severity,
        code: None,
        code_description: None,
        source: Some("typua".to_string()),
        message: diagnostic.message,
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

fn lsp_range_from_text(range: TextRange) -> Range {
    Range {
        start: Position {
            line: range.start.line.saturating_sub(1) as u32,
            character: range.start.character.saturating_sub(1) as u32,
        },
        end: Position {
            line: range.end.line.saturating_sub(1) as u32,
            character: range.end.character.saturating_sub(1) as u32,
        },
    }
}

fn default_range() -> Range {
    Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 0,
            character: 0,
        },
    }
}

fn uri_to_path(uri: &Url) -> PathBuf {
    if let Ok(path) = uri.to_file_path() {
        path
    } else {
        Path::new(uri.path()).to_path_buf()
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
