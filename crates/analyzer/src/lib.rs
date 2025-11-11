use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
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
    pub fn analyze(&self, path: &PathBuf, version: &LuaVersion) -> anyhow::Result<()> {
        let mut f = File::open(path)?;
        let mut content = String::new();
        f.read_to_string(&mut content)?;
        let (ast, errors) = parse(&content, *version);
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
