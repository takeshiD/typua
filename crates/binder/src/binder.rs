use typua_parser::ast::{Stmt, TypeAst};
use typua_ty::TypuaError;

use crate::flowgraph::FlowGraph;
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
                Stmt::LocalAssign(local_assign) => {}
                _ => unimplemented!(),
            }
        }
    }
}
