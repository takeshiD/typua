use crate::annotation::{AnnotationInfo, concat_tokens, parse_annotation};
use typua_span::{Position, Span};

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAst {
    pub block: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
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
    pub vars: Vec<Variable>,
    pub exprs: Vec<Expression>,
    pub annotates: Vec<AnnotationInfo>,
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
    Number {
        span: Span,
        val: String,
    },
    String {
        span: Span,
        val: String,
    },
    Boolean {
        span: Span,
        val: String,
    },
    Nil {
        span: Span,
    },
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
        params: Vec<Param>,
        body: Block,
    },
    FunctionCall(FunctionCall),
    Var {
        var: Var,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub span: Span,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add(Span),
    Sub(Span),
    Mul(Span),
    Div(Span),
    Mod(Span),
    And(Span),
    Or(Span),
    GreaterThan(Span),
    GreaterThanEqual(Span),
    LessThan(Span),
    LessThanEqual(Span),
    Equal(Span),
    NotEqual(Span),
    TwoDots(Span),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    Minus(Span),
    Not(Span),
    Hash(Span),
    Tilde(Span),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Var {
    pub span: Span,
    pub symbol: String,
}

impl From<full_moon::ast::Ast> for TypeAst {
    fn from(ast: full_moon::ast::Ast) -> Self {
        Self {
            block: Block::from(ast.nodes().clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    pub name: String,
    pub span: Span,
}

impl From<full_moon::ast::Block> for Block {
    fn from(block: full_moon::ast::Block) -> Self {
        let mut stmts = Vec::new();
        for stmt in block.stmts() {
            stmts.push(Stmt::from(stmt.clone()));
        }
        Self { stmts }
    }
}

impl From<full_moon::ast::Stmt> for Stmt {
    fn from(stmt: full_moon::ast::Stmt) -> Self {
        match stmt {
            full_moon::ast::Stmt::Assignment(_assign) => unimplemented!(),
            full_moon::ast::Stmt::LocalAssignment(local_assign) => {
                let leading_tribia = local_assign.local_token().leading_trivia();
                let ann_content = concat_tokens(leading_tribia);
                let annotates = parse_annotation(&ann_content);
                let vars: Vec<Variable> = local_assign
                    .names()
                    .iter()
                    .map(|t| Variable {
                        name: t.token().to_string(),
                        span: Span {
                            start: Position::from(t.start_position()),
                            end: Position::from(t.end_position()),
                        },
                    })
                    .collect();
                let exprs: Vec<Expression> = local_assign
                    .expressions()
                    .iter()
                    .map(|e| Expression::from(e.clone()))
                    .collect();
                Stmt::LocalAssign(LocalAssign {
                    vars,
                    exprs,
                    annotates,
                })
            }
            // full_moon::ast::Stmt::FunctionDeclaration(func_dec) => unimplemented!(),
            _ => unimplemented!(),
        }
    }
}

impl From<full_moon::ast::Expression> for Expression {
    fn from(expr: full_moon::ast::Expression) -> Self {
        match expr {
            full_moon::ast::Expression::Number(tkn) => Expression::Number {
                span: Span {
                    start: Position::from(tkn.start_position()),
                    end: Position::from(tkn.end_position()),
                },
                val: tkn.token().to_string(),
            },
            full_moon::ast::Expression::String(tkn) => Expression::String {
                span: Span {
                    start: Position::from(tkn.start_position()),
                    end: Position::from(tkn.end_position()),
                },
                val: tkn.token().to_string(),
            },
            full_moon::ast::Expression::Symbol(tkn) => match tkn.token_type() {
                full_moon::tokenizer::TokenType::Symbol { symbol } => match symbol {
                    full_moon::tokenizer::Symbol::False => Expression::Boolean {
                        span: Span {
                            start: Position::from(tkn.start_position()),
                            end: Position::from(tkn.end_position()),
                        },
                        val: tkn.token().to_string(),
                    },
                    full_moon::tokenizer::Symbol::Nil => Expression::Nil {
                        span: Span {
                            start: Position::from(tkn.start_position()),
                            end: Position::from(tkn.end_position()),
                        },
                    },
                    full_moon::tokenizer::Symbol::True => Expression::Boolean {
                        span: Span {
                            start: Position::from(tkn.start_position()),
                            end: Position::from(tkn.end_position()),
                        },
                        val: tkn.token().to_string(),
                    },
                    _ => unimplemented!(),
                },
                _ => unimplemented!(),
            },
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
            full_moon::ast::Expression::Var(var) => match var {
                full_moon::ast::Var::Expression(_expr) => {
                    unimplemented!()
                }
                full_moon::ast::Var::Name(tkn) => Expression::Var {
                    var: Var {
                        span: Span::from(tkn.clone()),
                        symbol: tkn.token().to_string(),
                    },
                },
                _ => unimplemented!(),
            },
            full_moon::ast::Expression::Parentheses { expression, .. } => {
                Expression::from(*expression)
            }
            full_moon::ast::Expression::Function(ann_func) => {
                let mut params = Vec::new();
                for param in ann_func.body().parameters().iter() {
                    match param {
                        full_moon::ast::Parameter::Ellipsis(_) => unimplemented!(),
                        full_moon::ast::Parameter::Name(tkn) => params.push(Param {
                            span: Span::new(
                                Position::from(tkn.start_position()),
                                Position::from(tkn.end_position()),
                            ),
                            name: tkn.to_string(),
                        }),
                        _ => unimplemented!(),
                    };
                }
                Expression::Function {
                    params,
                    body: Block::from(ann_func.body().block().clone()),
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
            full_moon::ast::BinOp::Plus(tkn)  => BinOp::Add(Span::from(tkn.clone())),
            full_moon::ast::BinOp::Minus(tkn) => BinOp::Sub(Span::from(tkn.clone())),
            full_moon::ast::BinOp::Star(tkn)  => BinOp::Mul(Span::from(tkn.clone())),
            full_moon::ast::BinOp::Slash(tkn) => BinOp::Div(Span::from(tkn.clone())),
            full_moon::ast::BinOp::Percent(tkn) => BinOp::Mod(Span::from(tkn.clone())),
            full_moon::ast::BinOp::TwoDots(tkn) => BinOp::TwoDots(Span::from(tkn.clone())),
            full_moon::ast::BinOp::And(tkn) => BinOp::And(Span::from(tkn.clone())),
            full_moon::ast::BinOp::Or(tkn) => BinOp::Or(Span::from(tkn.clone())),
            full_moon::ast::BinOp::GreaterThan(tkn) => BinOp::GreaterThan(Span::from(tkn.clone())),
            full_moon::ast::BinOp::GreaterThanEqual(tkn) => BinOp::GreaterThanEqual(Span::from(tkn.clone())),
            full_moon::ast::BinOp::LessThan(tkn) => BinOp::LessThan(Span::from(tkn.clone())),
            full_moon::ast::BinOp::LessThanEqual(tkn) => BinOp::LessThanEqual(Span::from(tkn.clone())),
            full_moon::ast::BinOp::TwoEqual(tkn) => BinOp::Equal(Span::from(tkn.clone())),
            full_moon::ast::BinOp::TildeEqual(tkn) => BinOp::NotEqual(Span::from(tkn.clone())),
            _ => unimplemented!()
        }
    }
}

impl From<full_moon::ast::UnOp> for UnOp {
    #[rustfmt::skip]
    fn from(unop: full_moon::ast::UnOp) -> Self {
        match unop {
            full_moon::ast::UnOp::Minus(tkn)  => UnOp::Minus(Span::from(tkn.clone())),
            full_moon::ast::UnOp::Hash(tkn)   => UnOp::Hash(Span::from(tkn.clone())),
            full_moon::ast::UnOp::Not(tkn)    => UnOp::Not(Span::from(tkn.clone())),
            _ => unimplemented!()
        }
    }
}
