use std::collections::HashMap;

// use full_moon as fm;
use full_moon::ast;
use full_moon::node::Node;

use super::types::{AnnotatedType, AnnotationIndex};
use crate::diagnostics::TextRange;

#[derive(Debug, Clone)]
pub struct Program {
    pub block: Block,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    LocalAssign(LocalAssign),
    Assign(Assign),
    Function(Function),
    Return(ReturnStmt),
    Unknown(TextRange),
}

#[derive(Debug, Clone)]
pub struct LocalAssign {
    pub names: Vec<String>,
    pub values: Vec<Expr>,
    pub ann: HashMap<String, AnnotatedType>,
    pub range: TextRange,
}

#[derive(Debug, Clone)]
pub struct Assign {
    pub targets: Vec<Expr>,
    pub values: Vec<Expr>,
    pub range: TextRange,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: Option<String>,
    pub params: Vec<String>,
    pub param_types: HashMap<String, AnnotatedType>,
    pub returns: Vec<AnnotatedType>,
    pub body: Block,
    pub range: TextRange,
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub values: Vec<Expr>,
    pub range: TextRange,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(String),
    String(String),
    TableConstructor(Vec<(Option<String>, Expr)>),
    Name(String),
    Field(Box<Expr>, String),
    BinOp(Box<Expr>, String, Box<Expr>),
    Call(String, Vec<Expr>),
    Unknown(TextRange),
}

fn token_range<T: Node>(node: &T) -> TextRange {
    let (start, end) = (node.start_position(), node.end_position());
    let start = start
        .map(Into::into)
        .unwrap_or(crate::diagnostics::TextPosition {
            line: 0,
            character: 0,
        });
    let end = end.map(Into::into).unwrap_or(start);
    crate::diagnostics::TextRange { start, end }
}

pub fn build_typed_ast(source: &str, ast: &ast::Ast, annotations: &AnnotationIndex) -> Program {
    let block = to_block(source, ast.nodes(), annotations);
    Program { block }
}

fn to_block(source: &str, block: &ast::Block, annotations: &AnnotationIndex) -> Block {
    let mut out = Vec::new();
    for s in block.stmts() {
        out.push(to_stmt(source, s, annotations));
    }
    if let Some(last) = block.last_stmt()
        && let ast::LastStmt::Return(ret) = last
    {
        let vals = ret.returns().iter().map(to_expr).collect();
        out.push(Stmt::Return(ReturnStmt {
            values: vals,
            range: token_range(last),
        }));
    }
    Block { stmts: out }
}

fn to_stmt(_source: &str, s: &ast::Stmt, annotations: &AnnotationIndex) -> Stmt {
    match s {
        ast::Stmt::LocalAssignment(assign) => {
            let names: Vec<String> = assign
                .names()
                .iter()
                .map(|n| n.token().to_string())
                .collect();
            let mut ann = HashMap::new();
            // アノテーションは直前行に付与される前提。AnnotationIndex.by_lineはpub。
            // ここでは安全にクローンされたインデックスから参照のみ行う。
            if let Some(pos) = assign.start_position()
                && let Some(vec) = annotations.by_line.get(&pos.line())
            {
                for a in vec.iter() {
                    if let Some(name) = &a.name {
                        ann.insert(name.clone(), a.ty.clone());
                    }
                }
            }
            let values = assign
                .expressions()
                .pairs()
                .map(|p| to_expr(p.value()))
                .collect();
            Stmt::LocalAssign(LocalAssign {
                names,
                values,
                ann,
                range: token_range(assign),
            })
        }
        ast::Stmt::Assignment(assign) => {
            let targets = assign
                .variables()
                .pairs()
                .map(|p| to_expr_var(p.value()))
                .collect();
            let values = assign
                .expressions()
                .pairs()
                .map(|p| to_expr(p.value()))
                .collect();
            Stmt::Assign(Assign {
                targets,
                values,
                range: token_range(assign),
            })
        }
        ast::Stmt::FunctionDeclaration(f) => {
            let name: Option<String> = None;
            let params: Vec<String> = Vec::new();
            let mut param_types = HashMap::new();
            let mut returns = Vec::new();
            let line = f.function_token().token().start_position().line();
            if let Some(vec) = annotations.by_line.get(&line) {
                for a in vec.iter() {
                    match a.usage {
                        super::types::AnnotationUsage::Param => {
                            if let Some(name) = &a.name {
                                param_types.insert(name.clone(), a.ty.clone());
                            }
                        }
                        super::types::AnnotationUsage::Return => {
                            returns.push(a.ty.clone());
                        }
                        _ => {}
                    }
                }
            }
            let body = to_block("", f.body().block(), annotations);
            Stmt::Function(Function {
                name,
                params,
                param_types,
                returns,
                body,
                range: token_range(f),
            })
        }
        _ => Stmt::Unknown(token_range(s)),
    }
}

fn to_expr(e: &ast::Expression) -> Expr {
    match e {
        ast::Expression::Number(n) => Expr::Number(n.token().to_string()),
        ast::Expression::String(s) => Expr::String(s.to_string()),
        ast::Expression::TableConstructor(_t) => Expr::TableConstructor(Vec::new()),
        ast::Expression::Var(var) => to_expr_var(var),
        ast::Expression::BinaryOperator { lhs, binop, rhs } => Expr::BinOp(
            Box::new(to_expr(lhs)),
            binop.to_string(),
            Box::new(to_expr(rhs)),
        ),
        ast::Expression::FunctionCall(call) => {
            let name = call.prefix().to_string();
            let args = Vec::new();
            Expr::Call(name, args)
        }
        _ => Expr::Unknown(token_range(e)),
    }
}

fn to_expr_var(var: &ast::Var) -> Expr {
    match var {
        ast::Var::Name(n) => Expr::Name(n.token().to_string()),
        ast::Var::Expression(_e) => Expr::Unknown(token_range(var)),
        _ => Expr::Unknown(token_range(var)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typechecker::types::AnnotationIndex;

    #[test]
    fn build_minimal_typed_ast() {
        let src = r#"
        ---@type number
        local x = 1
        "#;
        let ast = full_moon::parse(src).expect("parse");
        let (ann, _) = AnnotationIndex::from_source(src);
        let prog = build_typed_ast(src, &ast, &ann);
        assert_eq!(prog.block.stmts.len(), 1);
        match &prog.block.stmts[0] {
            Stmt::LocalAssign(l) => {
                assert_eq!(l.names[0], "x");
                assert!(!l.ann.contains_key("x")); // 名前付き---@param/@typeは直付けしないがIndexに保持
            }
            _ => panic!("expected local assign"),
        }
    }
}
