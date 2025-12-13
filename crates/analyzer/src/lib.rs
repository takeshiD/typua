use typua_binder::Binder;
use typua_checker::Checker;
use typua_config::LuaVersion;
use typua_lsp::handler::{
    CompletionResult, DiagnosticsResult, GotoDefinitionResult, HoverResult, InlayHintsResult,
    LspHandler, ReferencesResult, RenameResult,
};
use typua_parser::parse;
use typua_ty::{diagnostic::Diagnostic, typeinfo::TypeInfo};
use typua_workspace::WorkspaceManager;
//
#[derive(Debug, Clone)]
pub struct AnalyzeResult {
    pub type_infos: Vec<TypeInfo>,
    pub diagnotics: Vec<Diagnostic>,
}

#[derive(Clone, Default)]
pub struct Analyzer<W>
where
    W: WorkspaceManager,
{
    workspace_manager: W,
}

impl<W: WorkspaceManager> Analyzer<W> {
    pub fn new(workspace_manager: W) -> Self {
        Self { workspace_manager }
    }
    pub fn analyze(&self, _uri: &str, content: &str, version: &LuaVersion) -> AnalyzeResult {
        let (ast, _errors) = parse(content, *version);
        let mut binder = Binder::new();
        binder.bind(&ast);
        let env = binder.get_env();
        println!("Env: {:#?}", env);
        let checker = Checker::new(env);
        let check_result = checker.typecheck(&ast);
        println!("Report: {:#?}", check_result);
        AnalyzeResult {
            type_infos: check_result.type_infos,
            diagnotics: check_result.diagnostics,
        }
    }
}

impl<W: WorkspaceManager> LspHandler for Analyzer<W> {
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
