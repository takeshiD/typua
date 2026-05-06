use crate::symbol::Symbol;
use typua_syntax::cst;
use typua_types::TypeKind;

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalBinding {
    name: Symbol,
    ann: TypeAnnotation,
    select_index: usize,
}
#[derive(Debug, Clone, PartialEq)]
pub struct LocalBinding {
    pub name: Symbol,
    pub ann: TypeAnnotation,
    pub select_index: usize,
}

#[derive(PartialEq, Clone)]
pub struct StmtId(usize);

impl StmtId {
    pub fn id(&self) -> usize {
        self.0
    }
}

impl std::fmt::Display for StmtId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "StmtId({})", self.id())
    }
}

impl std::fmt::Debug for StmtId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match f.alternate() {
            true => write!(f, "StmtId({})", self.id()),
            false => write!(f, "StmtId({})", self.id()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
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

#[derive(PartialEq, Clone)]
pub struct ExprId(usize);

impl ExprId {
    pub fn id(&self) -> usize {
        self.0
    }
}

// pub type ExprId = Idx<Expr>;
impl std::fmt::Display for ExprId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ExprId({})", self.id())
    }
}
impl std::fmt::Debug for ExprId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match f.alternate() {
            true => write!(f, "ExprId({})", self.id()),
            false => write!(f, "ExprId({})", self.id()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
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

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            Self::Number => "number",
            Self::String => "string",
            Self::Boolean => "boolean",
            Expr::Nil => "nil",
            Expr::BinaryOperator => "binop",
            Expr::UnaryOperator => "unop",
            Expr::Function { params, body } => "func",
            Expr::FunctionCall => todo!(),
            Expr::Var => todo!(),
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum TypeAnnotation {
    Unannotated,
    Number,
    Named(String),
}

#[derive(Debug)]
pub struct HirBody {
    stmts: Vec<Stmt>,
    exprs: Vec<Expr>,
    roots: Vec<StmtId>,
}

impl HirBody {
    pub fn new() -> Self {
        Self {
            exprs: Vec::new(),
            stmts: Vec::new(),
            roots: Vec::new(),
        }
    }
    fn alloc_stmt(&mut self, stmt: Stmt) -> StmtId {
        self.stmts.push(stmt);
        StmtId(self.stmts.len() - 1)
    }
    fn alloc_expr(&mut self, expr: Expr) -> ExprId {
        self.exprs.push(expr);
        ExprId(self.exprs.len() - 1)
    }
    pub fn find_stmt(&self, stmt_id: &StmtId) -> Option<&Stmt> {
        self.stmts.get(stmt_id.id())
    }
    pub fn find_expr(&self, expr_id: &ExprId) -> Option<&Expr> {
        self.exprs.get(expr_id.id())
    }
    pub fn roots(&self) -> impl Iterator<Item = &StmtId> {
        self.roots.iter()
    }
    // pub fn find_stmt(&self, id: &StmtId) -> &Stmt {
    //     &self.stmts[id.id()]
    // }
    // pub fn find_expr(&self, id: &ExprId) -> &Expr {
    //     &self.exprs[id.id()]
    // }
    // entry point for lowering
    pub fn lower(&mut self, cst: &cst::Cst) {
        self.roots = self.lower_block(&cst.block);
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
        Some(self.alloc_stmt(s))
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
                let symbol = Symbol::new(v.name.clone());
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
                params: params
                    .iter()
                    .map(|p| Symbol::new(p.name.clone()))
                    .collect(),
                body: self.lower_func_body(body),
            },
            cst::Expression::Var { var } => Expr::Var,
            _ => unimplemented!(),
        };
        self.alloc_expr(e)
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
    use typua_types::TypeKind;
    #[test]
    fn test_hir() {
        // ---@type number
        // local x = 1
        // local y = x
        let stmts = vec![
            cst::Stmt::LocalAssign(cst::LocalAssign {
                vars: vec![cst::Variable {
                    name: "x".to_string(),
                    span: Span::new(6, 7),
                }],
                exprs: vec![cst::Expression::Number {
                    span: Span::new(10, 11),
                    val: "1".to_string(),
                }],
                annotates: vec![cst::AnnotationInfo {
                    span: Span::new(9, 15),
                    tag: cst::AnnotationTag::Type(TypeKind::Number),
                }],
            }),
            cst::Stmt::LocalAssign(cst::LocalAssign {
                vars: vec![cst::Variable {
                    name: "y".to_string(),
                    span: Span::new(6, 7),
                }],
                exprs: vec![cst::Expression::Number {
                    span: Span::new(10, 11),
                    val: "1".to_string(),
                }],
                annotates: vec![cst::AnnotationInfo {
                    span: Span::new(9, 15),
                    tag: cst::AnnotationTag::Type(TypeKind::Number),
                }],
            }),
        ];
        let block = cst::Block { stmts };
        let mut hir = HirBody::new();
        // expected roots
        let actual = hir.lower_block(&block);
        let expected: Vec<StmtId> = vec![StmtId(0), StmtId(1)];
        assert_eq!(actual, expected,);
    }
}
