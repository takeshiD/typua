mod db;
mod files;
mod paser;

use crate::db::RootDatabase;

use typua_binder::Binder;
use typua_checker::Checker;
use typua_config::LuaVersion;
use typua_lsp::handler::LspHandler;
use typua_parser::parse;
use typua_ty::diagnostic::Diagnostic;
use typua_ty::typeinfo::TypeInfo;
//
#[derive(Debug, Clone)]
pub struct AnalyzeResult {
    pub type_infos: Vec<TypeInfo>,
    pub diagnotics: Vec<Diagnostic>,
}

#[derive(Clone, Default)]
pub struct Analyzer {
    db: RootDatabase,
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            db: RootDatabase::default(),
        }
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
