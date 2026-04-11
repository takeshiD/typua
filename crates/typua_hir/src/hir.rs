use std::fmt::write;

use crate::arena::{Arena, Idx};

use typua_syntax::cst;
use typua_types::TypeKind;

#[derive(Debug, PartialEq)]
struct Symbol(String);

#[derive(Debug, PartialEq)]
struct GlobalBinding {
    name: Symbol,
    ann: TypeAnnotation,
    select_index: usize,
}
#[derive(Debug, PartialEq)]
struct LocalBinding {
    name: Symbol,
    ann: TypeAnnotation,
    select_index: usize,
}

// struct StmtId(u32);
type StmtId = Idx<Stmt>;
impl std::fmt::Display for Idx<Stmt> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "StmtId({})", self.raw())
    }
}

impl std::fmt::Debug for Idx<Stmt> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "StmtId({})", self.raw())
        } else {
            write!(f, "StmtId({})", self.raw())
        }
    }
}

#[derive(Debug, PartialEq)]
enum Stmt {
    Assign {
        binds: Vec<GlobalBinding>,
        inits: Vec<ExprId>,
    },
    LocalAssign {
        binds: Vec<LocalBinding>,
        inits: Vec<ExprId>,
    },
    FunctionCall,
}

// struct ExprId(u32);
type ExprId = Idx<Expr>;
impl std::fmt::Display for Idx<Expr> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ExprId({})", self.raw())
    }
}
impl std::fmt::Debug for Idx<Expr> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "ExprId({})", self.raw())
        } else {
            write!(f, "ExprId({})", self.raw())
        }
    }
}

#[derive(Debug, PartialEq)]
enum Expr {
    Number,
    String,
    Boolean,
    Nil,
    BinaryOperator,
    UnaryOperator,
    Function {
        params: Vec<Symbol>,
        body: Vec<StmtId>,
    },
    FunctionCall,
    Var,
}

#[derive(Debug, PartialEq)]
enum TypeAnnotation {
    Unannotated,
    Number,
    Named(String),
}

#[derive(Debug)]
pub struct HirBody {
    stmts: Arena<Stmt>,
    exprs: Arena<Expr>,
    root: Vec<StmtId>,
}

impl HirBody {
    pub fn new() -> Self {
        Self {
            exprs: Arena::new(),
            stmts: Arena::new(),
            root: Vec::new(),
        }
    }
    pub fn lower(&mut self, cst: &cst::Cst) {
        self.root = self.lower_block(&cst.block);
    }
    fn lower_block(&mut self, block: &cst::Block) -> Vec<StmtId> {
        let stmts: Vec<StmtId> = block
            .stmts
            .iter()
            .filter_map(|s| self.lower_stmt(s))
            .collect();
        // TODO: push last_stmt
        stmts
    }
    fn lower_stmt(&mut self, stmt: &cst::Stmt) -> Option<StmtId> {
        let s = match stmt {
            cst::Stmt::LocalAssign(local_assign) => self.lower_local_assign(local_assign),
            // cst::Stmt::LocalFunction(local_func) => {}
            _ => unimplemented!(),
        };
        Some(self.stmts.alloc(s))
    }
    fn lower_local_assign(&mut self, local_assign: &cst::LocalAssign) -> Stmt {
        let binds: Vec<LocalBinding> = local_assign
            .vars
            .iter()
            .enumerate()
            .map(|(idx, v)| {
                let ann = match local_assign.annotates.get(idx) {
                    None => TypeAnnotation::Unannotated,
                    Some(cst::AnnotationInfo { tag, .. }) => match tag {
                        cst::AnnotationTag::Type(TypeKind::Number) => TypeAnnotation::Number,
                        cst::AnnotationTag::Type(TypeKind::Named(name)) => {
                            TypeAnnotation::Named(name.name())
                        }
                        _ => unimplemented!(),
                    },
                };
                let symbol = Symbol(v.name.to_string());
                LocalBinding {
                    name: symbol,
                    ann,
                    select_index: idx,
                }
            })
            .collect();
        let expr_ids = local_assign
            .exprs
            .iter()
            .map(|e| self.lower_expr(e))
            .collect();
        Stmt::LocalAssign {
            binds,
            inits: expr_ids,
        }
    }
    fn lower_expr(&mut self, expr: &cst::Expression) -> ExprId {
        let e = match expr {
            cst::Expression::Number { val, .. } => Expr::Number,
            cst::Expression::String { val, .. } => Expr::String,
            cst::Expression::Boolean { val, .. } => Expr::Boolean,
            cst::Expression::Function { params, body } => Expr::Function {
                params: params.iter().map(|p| Symbol(p.name.clone())).collect(),
                body: self.lower_func_body(body),
            },
            cst::Expression::Var { var } => Expr::Var,
            _ => unimplemented!(),
        };
        self.exprs.alloc(e)
    }
    fn lower_func_body(&mut self, body: &cst::Block) -> Vec<StmtId> {
        self.lower_block(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use typua_syntax::cst;
    use typua_syntax::span::Span;
    #[test]
    fn test_hir() {
        // local x = 1
        let stmts = vec![cst::Stmt::LocalAssign(cst::LocalAssign {
            vars: vec![
                cst::Variable {
                    name: "x".to_string(),
                    span: Span::new(6, 7),
                },
                cst::Variable {
                    name: "y".to_string(),
                    span: Span::new(6, 7),
                },
            ],
            exprs: vec![cst::Expression::Number {
                span: Span::new(10, 11),
                val: "1".to_string(),
            }],
            annotates: vec![],
        })];
        let block = cst::Block { stmts };
        let mut hir = HirBody::new();
        let actual = hir.lower_block(&block);
        let expected: Vec<StmtId> = vec![Idx::new(0)];
        println!("##### Statements ######");
        println!("{:#?}", hir.stmts);
        println!("##### Expressions ######");
        println!("{:#?}", hir.exprs);
        println!("##### Root ######");
        println!("{:#?}", actual);
        assert_eq!(actual, expected,);
    }
}
