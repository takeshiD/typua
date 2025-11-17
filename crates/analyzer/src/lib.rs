use typua_binder::Binder;
use typua_checker::Checker;
use typua_config::LuaVersion;
use typua_parser::parse;
use typua_ty::diagnostic::Diagnostic;
// use typua_ty::diagnostic::Diagnostic;
//
#[derive(Debug, Clone)]
pub struct AnalyzeResult {
    // type_info: TypeInfo,
    pub diagnotics: Vec<Diagnostic>,
}

#[derive(Debug, Default)]
pub struct Analyzer {}

impl Analyzer {
    pub fn new() -> Self {
        Self {}
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
            diagnotics: check_result.diagnostics,
        }
    }
}
