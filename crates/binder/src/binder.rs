use crate::typeenv::Symbol;
use itertools::{EitherOrBoth, Itertools};
use typua_parser::annotation::AnnotationTag;
use typua_parser::ast::{Stmt, TypeAst};
use typua_ty::TypeKind;

use crate::typeenv::TypeEnv;

#[derive(Debug, Clone, Default)]
pub struct Binder {
    pub type_env: TypeEnv,
}

impl Binder {
    pub fn new() -> Self {
        Self {
            type_env: TypeEnv::new(),
            // flowgraph: FlowGraph::new(),
        }
    }
    pub fn get_env(&self) -> TypeEnv {
        self.type_env.clone()
    }
    pub fn bind(&mut self, ast: &TypeAst) {
        for stmt in ast.block.stmts.iter() {
            match stmt {
                Stmt::LocalAssign(local_assign) => {
                    for pair in local_assign
                        .vars
                        .iter()
                        .zip_longest(local_assign.annotates.iter())
                    {
                        match pair {
                            EitherOrBoth::Both(var, ann) => {
                                let _ = match &ann.tag {
                                    AnnotationTag::Type(ty) => {
                                        self.type_env.insert(&Symbol::new(var.name.clone()), ty)
                                    }
                                    _ => unimplemented!(),
                                };
                            }
                            EitherOrBoth::Left(var) => {
                                let _ = self
                                    .type_env
                                    .insert(&Symbol::new(var.name.clone()), &TypeKind::Any);
                            }
                            EitherOrBoth::Right(_ann) => (),
                        }
                    }
                }
                _ => unimplemented!(),
            }
        }
    }
}
