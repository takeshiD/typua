use typua_binder::Binder;
use typua_checker::Checker;
use typua_config::LuaVersion;
use typua_parser::parse;
// use typua_ty::diagnostic::Diagnostic;

#[derive(Debug, Default)]
pub struct Analyzer {}

impl Analyzer {
    pub fn new() -> Self {
        Self {}
    }
    pub fn analyze(&self, content: &str, version: &LuaVersion) -> anyhow::Result<()> {
        let (ast, errors) = parse(content, *version);
        let mut binder = Binder::new();
        binder.bind(&ast);
        let env = binder.get_env();
        println!("Env: {:#?}", env);
        let checker = Checker::new(env);
        let report = checker.typecheck(&ast);
        println!("Report: {:#?}", report);
        println!("Errors: {:#?}", errors);
        Ok(())
    }
}
