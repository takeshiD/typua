use crate::diagnostic::{Diagnostic, IssueKind};
use crate::hir::HirBody;
use crate::infer::Inference;
use typua_config::LuaVersion;
use typua_syntax::{cst::Cst, parse};

#[derive(Debug)]
pub struct LuaFile {
    tree: Cst,
    hir: HirBody,
    inference: Inference,
    diagnostics: Vec<Diagnostic>,
}

impl LuaFile {
    pub fn new(code: &str, lua_version: LuaVersion) -> Self {
        let (cst, errors) = parse(code, lua_version);
        Self {
            tree: cst,
            hir: HirBody::new(),
            inference: Inference::new(),
            diagnostics: errors
                .into_iter()
                .map(|e| {
                    Diagnostic::new(IssueKind::SyntaxError(e.error().clone()), e.span().clone())
                })
                .collect(),
        }
    }
    pub fn lower(&mut self) {
        self.hir.lower(&self.tree)
    }
    pub fn infer(&mut self) {
        for stmt_id in self.hir.roots() {
            if let Some(stmt) = self.hir.find_stmt(stmt_id) {
                self.inference.infer_stmt(stmt);
            }
        }
    }
    pub fn diagnostics(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unindent::unindent;
    // use pretty_assertions::assert_eq;
    #[test]
    fn test_file_lowering() {
        let content = unindent(
            r#"
        ---@type number
        local x = x
        "#,
        );
        let mut file = LuaFile::new(&content, LuaVersion::LuaJIT);
        println!("Parsed: {:#?}", file.tree);
        file.lower();
        println!("Lowering: {:#?}", file.hir);
        file.infer();
        println!("Inferred: {:#?}", file.inference);
    }
}
