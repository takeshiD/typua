use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use tracing::{Level, event};

use full_moon::Error as FullMoonError;
use tokio::sync::RwLock;
use tower_lsp::{
    Client, LanguageServer, LspService, Server, async_trait,
    jsonrpc::Result as LspResult,
    lsp_types::{
        CodeDescription, Diagnostic as LspDiagnostic, DiagnosticSeverity,
        DidChangeTextDocumentParams, DidOpenTextDocumentParams, Hover, HoverContents, HoverParams,
        HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams, InlayHint,
        InlayHintKind, InlayHintLabel, InlayHintParams, MarkupContent, MarkupKind, MessageType,
        NumberOrString, OneOf, Position, Range, ServerCapabilities, ServerInfo,
        TextDocumentContentChangeEvent, TextDocumentSyncCapability, TextDocumentSyncKind,
        TextDocumentSyncOptions, Url, WorkspaceFoldersServerCapabilities,
        WorkspaceServerCapabilities,
    },
};

use crate::checker::{self, TypeInfo};
use crate::cli::LspOptions;
use crate::diagnostics::{Diagnostic as CheckerDiagnostic, Severity, TextRange};
use crate::error::Result;

const VERSION: &str = env!("CARGO_PKG_VERSION");

use crate::typechecker::types::{AnnotationIndex, TypeRegistry};
use crate::workspace;

#[derive(Debug)]
pub struct TypuaLanguageServer {
    client: Client,
    _root: RwLock<PathBuf>,
    _config: Arc<crate::config::Config>,
    documents: RwLock<HashMap<Url, DocumentState>>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Hash)]
pub struct DocumentPosition {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
struct DocumentState {
    text: String,
    types: HashMap<DocumentPosition, TypeInfo>,
}

impl TypuaLanguageServer {
    pub fn new(client: Client, options: LspOptions) -> Self {
        Self {
            client,
            _root: RwLock::new(PathBuf::new()),
            _config: Arc::new(options.config),
            documents: RwLock::new(HashMap::new()),
        }
    }

    async fn update_document(&self, uri: Url, text: String) {
        let (diagnostics, types) = self.analyze_document(&uri, &text).await;

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

    async fn collect_workspace_registry(&self, current: &Path) -> TypeRegistry {
        let mut registry = TypeRegistry::default();
        let root = self._root.read().await;
        match workspace::collect_source_files(&root, self._config.as_ref()) {
            Ok(files) => {
                for path in files {
                    if path == current {
                        continue;
                    }
                    match fs::read_to_string(&path) {
                        Ok(source) => {
                            let (_, file_registry) = AnnotationIndex::from_source(&source);
                            registry.extend(&file_registry);
                        }
                        Err(error) => {
                            event!(
                                Level::WARN,
                                ?path,
                                ?error,
                                "failed to read workspace file when collecting registry"
                            );
                        }
                    }
                }
            }
            Err(error) => {
                event!(
                    Level::WARN,
                    ?error,
                    "failed to collect workspace files for registry"
                );
            }
        }
        registry
    }

    async fn analyze_document(
        &self,
        uri: &Url,
        text: &str,
    ) -> (Vec<LspDiagnostic>, HashMap<DocumentPosition, TypeInfo>) {
        match full_moon::parse(text) {
            Ok(ast) => {
                let path = uri_to_path(uri);
                let workspace_registry = self.collect_workspace_registry(path.as_path()).await;
                let result =
                    checker::check_ast_with_registry(&path, text, &ast, Some(&workspace_registry));
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
    async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
        let text_document_sync = TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
            open_close: Some(true),
            change: Some(TextDocumentSyncKind::FULL),
            will_save: Some(false),
            will_save_wait_until: Some(false),
            save: None,
        });
        if let Some(workspace_root) = params.workspace_folders
            && !workspace_root.is_empty()
            && let Some(ws) = workspace_root.first()
        {
            let mut root = self._root.write().await;
            *root = PathBuf::from(ws.uri.as_str());
        }
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "typua".to_string(),
                version: Some(VERSION.to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(text_document_sync),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                inlay_hint_provider: Some(OneOf::Left(true)),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        let log_msg = format!("initialized in {:?}", self._root);
        self.client
            .log_message(MessageType::INFO, log_msg.clone())
            .await;
        event!(Level::INFO, "{}", log_msg);
    }

    async fn shutdown(&self) -> LspResult<()> {
        let log_msg = format!("shutdown in {:?}", self._root);
        self.client
            .log_message(MessageType::INFO, log_msg.clone())
            .await;
        event!(Level::INFO, "{}", log_msg);
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let log_msg = format!("did open {} in {:?}", params.text_document.uri, self._root);
        self.client
            .log_message(MessageType::LOG, log_msg.clone())
            .await;
        event!(Level::DEBUG, "{}", log_msg);
        let text_document = params.text_document;
        self.update_document(text_document.uri, text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let log_msg = format!(
            "did change {} in {:?}",
            params.text_document.uri, self._root
        );
        self.client
            .log_message(MessageType::LOG, log_msg.clone())
            .await;
        event!(Level::DEBUG, "{}", log_msg);
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
        let log_msg = format!("did close {} in {:?}", params.text_document.uri, self._root);
        self.client
            .log_message(MessageType::LOG, log_msg.clone())
            .await;
        event!(Level::DEBUG, "{}", log_msg);
        self.remove_document(&params.text_document.uri).await;
    }

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let documents = self.documents.read().await;
        let log_msg = format!(
            "hover {} (line:{}, char:{}) in {:?}",
            uri, position.line, position.character, self._root
        );
        self.client
            .log_message(MessageType::LOG, log_msg.clone())
            .await;
        event!(Level::DEBUG, "{}", log_msg);
        if let Some(state) = documents.get(&uri) {
            let line = position.line as usize + 1;
            let character = position.character as usize + 1;
            if let Some(((start_line, start_char), entry)) =
                lookup_type_at(&state.types, line, character)
                && entry.ty != "unknown"
            {
                let contents = HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("**Type**\n`{}`", entry.ty),
                });
                let range = Some(Range {
                    start: Position {
                        line: start_line.saturating_sub(1) as u32,
                        character: start_char.saturating_sub(1) as u32,
                    },
                    end: Position {
                        line: entry.end_line.saturating_sub(1) as u32,
                        character: entry.end_character.saturating_sub(1) as u32,
                    },
                });
                return Ok(Some(Hover { contents, range }));
            }
        }
        Ok(Some(Hover {
            contents: HoverContents::Scalar(tower_lsp::lsp_types::MarkedString::String(
                "Not infered...".to_string(),
            )),
            range: None,
        }))
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> LspResult<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;
        let range = params.range;
        // LSP positions are 0-based; checker records positions as 1-based.
        let start_row = range.start.line as usize + 1;
        let end_row = range.end.line as usize + 1;
        let start_col = range.start.character as usize + 1;
        let end_col = range.end.character as usize + 1;

        let log_msg = format!(
            "inlay-hint {} (row:{}-{}, col:{}-{}) in {:?}",
            uri, start_row, end_row, start_col, end_col, self._root
        );
        self.client
            .log_message(MessageType::LOG, log_msg.clone())
            .await;
        event!(Level::DEBUG, "{}", log_msg);

        let documents = self.documents.read().await;
        let Some(state) = documents.get(&uri) else {
            return Ok(Some(Vec::new()));
        };

        let mut entries: Vec<_> = state.types.iter().collect();
        entries.sort_by(|a, b| a.0.row.cmp(&b.0.row));

        let mut hints = Vec::new();
        for (&DocumentPosition { row, col }, info) in entries {
            if !position_in_range(row, col, start_row, start_col, end_row, end_col) {
                let log_msg = format!(
                    "inlay-hint out-of-range {} (row:{}, col:{}) in {:?}",
                    uri, row, col, self._root
                );
                self.client
                    .log_message(MessageType::WARNING, log_msg.clone())
                    .await;
                event!(Level::WARN, "{}", log_msg);
                continue;
            }
            let position = Position {
                line: info.end_line.saturating_sub(1) as u32,
                character: info.end_character.saturating_sub(1) as u32,
            };
            hints.push(InlayHint {
                position,
                label: InlayHintLabel::String(format!(": {}", info.ty)),
                kind: Some(InlayHintKind::TYPE),
                text_edits: None,
                tooltip: None,
                padding_left: Some(false),
                padding_right: Some(true),
                data: None,
            });
        }

        Ok(Some(hints))
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
        code: Some(tower_lsp::lsp_types::NumberOrString::String(
            "error".to_string(),
        )),
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
        code: Some(NumberOrString::String("diagnostic".to_string())),
        code_description: Some(CodeDescription {
            href: Url::parse("https://example.com").expect("parse failed"),
        }),
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

fn lookup_type_at(
    types: &HashMap<DocumentPosition, TypeInfo>,
    line: usize,
    character: usize,
) -> Option<((usize, usize), TypeInfo)> {
    types
        .iter()
        .filter(|(pos, info)| {
            let start_line = pos.row;
            let start_char = pos.col;
            if line < start_line || line > info.end_line {
                return false;
            }
            if line == start_line && character < start_char {
                return false;
            }
            if line == info.end_line && character > info.end_character {
                return false;
            }
            true
        })
        .max_by_key(|(pos, _)| (pos.row, pos.col))
        .map(|(k, info)| ((k.row, k.col), info.clone()))
}

fn position_in_range(
    line: usize,
    character: usize,
    start_row: usize,
    start_col: usize,
    end_row: usize,
    end_col: usize,
) -> bool {
    if line < start_row || line > end_row {
        return false;
    }
    if line == start_row && character < start_col {
        return false;
    }
    if line == end_row && character > end_col {
        return false;
    }
    true
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

#[cfg(test)]
mod tests {
    use super::position_in_range;
    use tower_lsp::lsp_types::{Position, Range};

    #[test]
    fn position_in_range_handles_final_line_after_zero_based_conversion() {
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 1,
                character: 20,
            },
        };

        let start_row = range.start.line as usize + 1;
        let end_row = range.end.line as usize + 1;
        let start_col = range.start.character as usize + 1;
        let end_col = range.end.character as usize + 1;

        assert!(position_in_range(
            2, 7, start_row, start_col, end_row, end_col
        ));
    }

    #[test]
    fn position_in_range_excludes_rows_outside_bounds() {
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 10,
            },
        };

        let start_row = range.start.line as usize + 1;
        let end_row = range.end.line as usize + 1;
        let start_col = range.start.character as usize + 1;
        let end_col = range.end.character as usize + 1;

        assert!(!position_in_range(
            2, 5, start_row, start_col, end_row, end_col
        ));
    }
}
