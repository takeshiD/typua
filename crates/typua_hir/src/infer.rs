use crate::hir;
use crate::symbol::Symbol;
use std::collections::HashMap;


#[derive(Debug)]
pub struct InferenceResult {}
impl InferenceResult {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug)]
enum InferType {
    Number,
    String,
    Unresolved(hir::ExprId),
}

#[derive(Debug)]
pub struct Inference {
    env: HashMap<Symbol, InferType>,
}

#[derive(Debug)]
struct Issue {
    content: String,
}

impl Inference {
    pub fn new() -> Self {
        Inference {
            env: HashMap::new(),
        }
    }
    pub fn infer_stmt(&mut self, stmt: &hir::Stmt) -> InferenceResult {
        match stmt {
            hir::Stmt::LocalAssign { binds, inits } => {
                for (i, b) in binds.iter().enumerate() {
                    // let name = b.name.to_string();
                    let expr_id = inits.get(i).expect("failed to get");
                    self.env
                        .insert(b.name.clone(), InferType::Unresolved(expr_id.clone()));
                }
            }
            _ => todo!(),
        }
        InferenceResult::new()
    }
    pub fn resolve_var(&mut self, var: Symbol) {
        unimplemented!()
    }
    fn add_issue(&self, issue: Issue) {
        unimplemented!()
    }
}
