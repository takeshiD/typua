use crate::hir;
use std::collections::HashMap;

#[derive(Debug)]
enum InferType {
    Number,
    String,
    Unresolved(hir::ExprId),
}

#[derive(Debug)]
pub struct Inference {
    env: HashMap<String, InferType>,
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
    pub fn infer_stmt(&mut self, stmt: &hir::Stmt) {
        match stmt {
            hir::Stmt::LocalAssign { binds, inits } => {
                for (i, b) in binds.iter().enumerate() {
                    let name = b.name.to_string();
                    let expr_id = inits.get(i).expect("failed to get");
                    self.env
                        .insert(name, InferType::Unresolved(expr_id.clone()));
                }
            }
            _ => todo!(),
        }
    }
    fn add_issue(&self, issue: Issue) {
        unimplemented!()
    }
}
