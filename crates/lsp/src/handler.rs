use tracing::info;
pub struct HoverResult {}
pub struct GotoDefinitionResult {}
pub struct ReferencesResult {}
pub struct CompletionResult {}
pub struct RenameResult {}
pub struct DiagnosticsResult {}
pub struct InlayHintsResult {}

pub trait LspHandler: Send + Sync + 'static {
    fn hover(&self) -> Option<HoverResult>;
    fn goto_definition(&self) -> Option<GotoDefinitionResult>;
    fn references(&self) -> Option<ReferencesResult>;
    fn completion(&self) -> Option<CompletionResult>;
    fn rename(&self) -> Option<RenameResult>;
    fn diagnostics(&self) -> Option<DiagnosticsResult>;
    fn inlay_hints(&self) -> Option<InlayHintsResult>;
}

#[derive(Debug, Default, Clone)]
pub struct EmptyLspHandler {}

impl EmptyLspHandler {
    pub fn new() -> Self {
        info!("create empty handler");
        Self {}
    }
}

impl LspHandler for EmptyLspHandler {
    fn hover(&self) -> Option<HoverResult> {
        None
    }
    fn goto_definition(&self) -> Option<GotoDefinitionResult> {
        None
    }
    fn references(&self) -> Option<ReferencesResult> {
        None
    }
    fn completion(&self) -> Option<CompletionResult> {
        None
    }
    fn rename(&self) -> Option<RenameResult> {
        None
    }
    fn diagnostics(&self) -> Option<DiagnosticsResult> {
        None
    }
    fn inlay_hints(&self) -> Option<InlayHintsResult> {
        None
    }
}
