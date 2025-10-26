use std::collections::BTreeMap;

use im::Vector;

use crate::annotation::{AnnotationTag, concat_tokens, parse_annotation};
use crate::span::Span;
use crate::types::TypeKind;

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAst {
    pub block: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vector<Stmt>,
}

/// Statements
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Stmt {
    Assign(Assign),
    LocalAssign(LocalAssign),
    FunctionCall(FunctionCall),
    FunctionDeclaration(FunctionDeclaration),
    LocalFunction(LocalFunction),
    // If(If),
    // Do(Do),
    // While(While),
    // Repeat(Repeat),
    // Goto(Goto),
    // NumericFor(NumericFor),
    // GenericFor(GenericFor),
    // Label(Label),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assign {}

#[derive(Debug, Clone, PartialEq)]
/// x, y["a"], z[1] = 1, "hello", nil
/// ids are x, y["a"], z[1]
/// exprs are 1, "hello", nil
pub struct LocalAssign {
    vars: Vec<Variable>,
    exprs: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalFunction {}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCall {}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDeclaration {}

#[derive(Debug, Clone, PartialEq)]
pub struct If {}

#[derive(Debug, Clone, PartialEq)]
pub struct Do {}

#[derive(Debug, Clone, PartialEq)]
pub struct While {}

#[derive(Debug, Clone, PartialEq)]
pub struct Repeat {}

#[derive(Debug, Clone, PartialEq)]
pub struct Goto {}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericFor {}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericFor {}

#[derive(Debug, Clone, PartialEq)]
pub struct Label {}

/// Expression
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Number(LuaNumber),
    String(LuaString),
    Boolean(LuaBoolean),
    BinaryOperator {
        lhs: Box<Expression>,
        binop: BinOp,
        rhs: Box<Expression>,
    },
    UnaryOperator {
        unop: UnOp,
        expr: Box<Expression>,
    },
    Function {
        params: BTreeMap<String, TypeKind>,
        returns: Vec<TypeKind>,
    },
    FunctionCall(FunctionCall),
    Var(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
    GreaterThan,
    GreaterThanEqual,
    LessThan,
    LessThanEqual,
    Equal,
    NotEqual,
    Concat,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    Minus,
    Not,
    Hash,
    Tilde,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LuaNumber {
    val: f32,
    span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LuaString {
    val: String,
    span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LuaBoolean {
    val: bool,
    span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    name: String,
    annotated_type: TypeKind,
}

// impl From<full_moon::ast::Block> for Block {
//     fn from(block: full_moon::ast::Block) -> Self {
//         let mut stmts = Vector::new();
//         for stmt in block.stmts() {
//             stmts.push_back(Stmt::from(stmt));
//         }
//         Self {
//             block: Block { stmts },
//         }
//     }
// }

impl From<full_moon::ast::Stmt> for Stmt {
    fn from(stmt: full_moon::ast::Stmt) -> Self {
        match stmt {
            // full_moon::ast::Stmt::Assignment(assign) => unimplemented!(),
            full_moon::ast::Stmt::LocalAssignment(local_assign) => {
                let leading_tribia = local_assign.local_token().leading_trivia();
                let ann_content = concat_tokens(leading_tribia);
                let annotated_type = if let Some(ann_info) = parse_annotation(ann_content) {
                    match ann_info.tag {
                        AnnotationTag::Type(ty) => ty,
                        _ => TypeKind::Any,
                    }
                } else {
                    TypeKind::Any
                };
                let vars: Vec<Variable> = local_assign
                    .names()
                    .iter()
                    .map(|t| Variable {
                        name: t.token().to_string(),
                        annotated_type: annotated_type.clone(),
                    })
                    .collect();
                let exprs: Vec<Expression> = local_assign
                    .expressions()
                    .iter()
                    .map(|e| Expression::from(e.clone()))
                    .collect();
                Stmt::LocalAssign(LocalAssign { vars, exprs })
            }
            // full_moon::ast::Stmt::FunctionDeclaration(func_dec) => unimplemented!(),
            // full_moon::ast::Stmt::LocalFunction(local_func) => unimplemented!(),
            _ => unimplemented!(),
        }
    }
}

impl From<full_moon::ast::Expression> for Expression {
    fn from(expr: full_moon::ast::Expression) -> Self {
        match expr {
            full_moon::ast::Expression::BinaryOperator { lhs, binop, rhs } => {
                Expression::BinaryOperator {
                    lhs: Box::new(Expression::from(*lhs)),
                    binop: BinOp::from(binop),
                    rhs: Box::new(Expression::from(*rhs)),
                }
            }
            full_moon::ast::Expression::UnaryOperator { unop, expression } => {
                Expression::UnaryOperator {
                    unop: UnOp::from(unop),
                    expr: Box::new(Expression::from(*expression)),
                }
            }
            _ => unimplemented!(),
        }
    }
}

impl From<full_moon::ast::BinOp> for BinOp {
    #[rustfmt::skip]
    fn from(binop: full_moon::ast::BinOp) -> Self {
        match binop {
            full_moon::ast::BinOp::Plus(_)  => BinOp::Add,
            full_moon::ast::BinOp::Minus(_) => BinOp::Sub,
            full_moon::ast::BinOp::Star(_)  => BinOp::Mul,
            full_moon::ast::BinOp::Slash(_) => BinOp::Div,
            _ => unimplemented!()
        }
    }
}

impl From<full_moon::ast::UnOp> for UnOp {
    #[rustfmt::skip]
    fn from(unop: full_moon::ast::UnOp) -> Self {
        match unop {
            full_moon::ast::UnOp::Minus(_)  => UnOp::Minus,
            full_moon::ast::UnOp::Hash(_)   => UnOp::Hash,
            full_moon::ast::UnOp::Not(_)    => UnOp::Not,
            _ => unimplemented!()
        }
    }
}
