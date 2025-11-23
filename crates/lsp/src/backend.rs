use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionOptions, CompletionParams, CompletionResponse,
    Diagnostic as LspDiagnostic, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents,
    HoverParams, HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams,
    InlayHint, InlayHintParams, Location, MarkupContent, MarkupKind, MessageType, OneOf,
    Position as LspPosition, Range as LspRange, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, Url, WorkDoneProgressOptions,
};
use tower_lsp::{Client, LanguageServer};
use tracing::{debug, info};
use typua_analyzer::Analyzer;
use typua_config::LuaVersion;
use typua_span::Position;

#[derive(Debug)]
pub struct Backend {
    pub client: Client,
    pub analyzer: Analyzer,
    pub documents: Arc<RwLock<HashMap<Url, String>>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            analyzer: Analyzer::new(),
            documents: Arc::new(RwLock::new(HashMap::new())),
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
                    // TextDocumentSyncKind::INCREMENTAL,
                    TextDocumentSyncKind::FULL,
                )),
                inlay_hint_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: None,
                    trigger_characters: None,
                    all_commit_characters: None,
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: None,
                    },
                    completion_item: None,
                }),
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
        {
            let docs = Arc::clone(&self.documents);
            if let Ok(mut doc_map) = docs.write() {
                doc_map.insert(uri.clone(), content.clone());
            }
        }
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
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        info!("did change: {}", &uri);
        let content = params.content_changes[0].text.clone();
        {
            let docs = Arc::clone(&self.documents);
            if let Ok(mut doc_map) = docs.write() {
                doc_map.insert(uri.clone(), content.clone());
            }
        }
        let analyze_result = self
            .analyzer
            .analyze(uri.as_ref(), &content, &LuaVersion::Lua51);
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
    async fn inlay_hint(&self, params: InlayHintParams) -> LspResult<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;
        let content = {
            let docs = Arc::clone(&self.documents);
            if let Ok(doc_map) = docs.read() {
                doc_map.get(&uri).cloned()
            } else {
                None
            }
        };
        let inlay_hints: Vec<InlayHint> = match content {
            Some(content) => {
                info!("inlay hint: {} is Some", &uri);
                self.analyzer
                    .analyze(uri.as_ref(), &content, &LuaVersion::Lua51)
                    .type_infos
                    .iter()
                    .map(InlayHint::from)
                    .collect()
            }
            None => {
                info!("inlay hint: {} is None", &uri,);
                Vec::new()
            }
        };
        debug!("inlay hints: {:#?}", inlay_hints);
        Ok(Some(inlay_hints))
    }
    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let content = {
            let docs = Arc::clone(&self.documents);
            if let Ok(doc_map) = docs.read() {
                doc_map.get(&uri).cloned()
            } else {
                None
            }
        };
        let position = Position::from(params.text_document_position_params.position);
        info!("hover: {}", uri);
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: r#"""
                # Hello Hover
                [Github - typua](https://github.com/takeshiD/typua.git)
                """#
                .to_string(),
            }),
            range: None,
        }))
    }
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> LspResult<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let content = {
            let docs = Arc::clone(&self.documents);
            if let Ok(doc_map) = docs.read() {
                doc_map.get(&uri).cloned()
            } else {
                None
            }
        };
        let position = Position::from(params.text_document_position_params.position);
        info!("goto difinition: {}", uri);
        Ok(Some(GotoDefinitionResponse::Array(vec![
            Location {
                uri: uri.clone(),
                range: LspRange::new(LspPosition::new(0, 0), LspPosition::new(0, 10)),
            },
            Location {
                uri,
                range: LspRange::new(LspPosition::new(2, 0), LspPosition::new(2, 10)),
            },
        ])))
    }
    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        info!("completion {:#?}", params);
        Ok(Some(CompletionResponse::Array(vec![CompletionItem {
            label: "hello".to_string(),
            ..CompletionItem::default()
        }])))
    }
}
