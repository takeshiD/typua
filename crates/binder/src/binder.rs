use crate::typeenv::Symbol;
use typua_parser::annotation::AnnotationTag;
use typua_parser::ast::{Stmt, TypeAst};

use crate::typeenv::TypeEnv;

pub struct Binder {
    type_env: TypeEnv,
    // flowgraph: FlowGraph,
}

impl Binder {
    fn new() -> Self {
        Self {
            type_env: TypeEnv::new(),
            // flowgraph: FlowGraph::new(),
        }
    }
    fn bind(&mut self, ast: TypeAst) {
        for stmt in ast.block.stmts.iter() {
            match stmt {
                Stmt::LocalAssign(local_assign) => {
                    for (var, ann) in local_assign.vars.iter().zip(local_assign.annotates.iter()) {
                        let _ = match &ann.tag {
                            AnnotationTag::Type(ty) => {
                                self.type_env.insert(&Symbol::new(var.name.clone()), ty)
                            }
                            _ => unimplemented!(),
                        };
                    }
                }
                _ => unimplemented!(),
            }
        }
    }
}
