use crate::hir::HirBody;
use crate::infer::Inference;
use typua_config::LuaVersion;
use typua_syntax::{cst::Cst, parse};

struct LuaFile {
    content: String,
    tree: Option<Cst>,
    hir: HirBody,
    inference: Inference,
    issues: Vec<String>,
}

impl LuaFile {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            tree: None,
            hir: HirBody::new(),
            inference: Inference::new(),
            issues: Vec::new(),
        }
    }
    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }
    pub fn parse(&mut self) {
        let (tree, errors) = parse(&self.content, LuaVersion::LuaJIT);
        self.tree = Some(tree);
        self.issues.extend(errors.iter().map(|s| s.error().clone()));
    }
    pub fn lower(&mut self) {
        match &self.tree {
            Some(tree) => self.hir.lower(tree),
            None => unimplemented!(),
        }
    }
    pub fn infer(&mut self) {
        for stmt_id in self.hir.roots() {
            if let Some(stmt) = self.hir.find_stmt(stmt_id) {
                self.inference.infer_stmt(stmt);
            }
        }
    }
    pub fn resolve(&mut self) {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unindent::unindent;
    use pretty_assertions::assert_eq;
    #[test]
    fn test_file_lowering() {
        let mut file = LuaFile::new();
        let content = unindent(r#"
        ---@type number
        local x = x
        "#);
        file.set_content(content);
        file.parse();
        println!("Parsed: {:#?}", file.tree);
        file.lower();
        println!("Lowering: {:#?}", file.hir);
        file.infer();
        println!("Inferred: {:#?}", file.inference);
    }
}
