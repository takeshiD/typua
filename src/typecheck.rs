use anyhow::{Context, Result, anyhow};
use core::panic;
use full_moon::{
    ast::{Ast, BinOp, Block, Expression, LastStmt, Parameter, Stmt, UnOp, Var},
    tokenizer::{Symbol, TokenType},
};
use std::{arch::global_asm, collections::BTreeMap};

#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    Unknown,
    None,
    Any,
    Nil,
    Boolean,
    Number,
    String,
    Array,
    Table,
    Function(FunctionType),
    MultiValue(MultiValueType),
}

#[derive(Debug, PartialEq, Clone)]
pub struct FunctionType {
    params: Vec<Param>,
    ret_ty: Box<Option<Type>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Param {
    symbol: String,
    ty: Type,
}

#[derive(Debug, PartialEq, Clone)]
pub struct MultiValueType {
    types: Vec<Type>,
}

type TypeEnv = BTreeMap<String, Option<Type>>;

#[derive(Debug, Clone)]
pub struct TypeEnvStack {
    envs: BTreeMap<usize, TypeEnv>,
    depth: usize,
}

impl TypeEnvStack {
    pub fn new() -> Self {
        let mut envs = BTreeMap::new();
        let depth = 0;
        envs.insert(depth, BTreeMap::new());
        Self { envs, depth }
    }
    pub fn push_env(&mut self) {
        self.depth += 1;
        self.envs.insert(self.depth, BTreeMap::new());
    }
    pub fn pop_env(&mut self) {
        self.envs.remove(&self.depth);
        self.depth -= 1;
    }
    pub fn bind(&mut self, name: String, type_: Type) {
        if let Some(current_env) = self.envs.get_mut(&self.depth) {
            current_env.insert(name, Some(type_));
        } else {
            panic!(
                "EnvStack is empty: depth={}, len(envs)={}",
                self.depth,
                self.envs.len()
            );
        }
    }
    pub fn lookup(&self, name: &str) -> Option<Type> {
        for (_, env) in self.envs.iter().rev() {
            if let Some(ty) = env.get(name) {
                return ty.clone();
            }
        }
        None
    }
    pub fn is_empty(&self) -> bool {
        self.envs.is_empty()
    }
}

pub fn typecheck(
    _local_env: &mut TypeEnvStack,
    _global_env: &mut TypeEnvStack,
    _ast: &Ast,
) -> Option<Type> {
    unimplemented!()
}

pub fn typecheck_block(
    local_env: &mut TypeEnvStack,
    global_env: &mut TypeEnvStack,
    block: &Block,
) -> Option<Type> {
    for stmt in block.stmts() {
        let _ = typecheck_stmt(local_env, global_env, stmt);
    }
    if let Some(last_stmt) = block.last_stmt() {
        typecheck_last_stmt(local_env, global_env, last_stmt)
    } else {
        None
    }
}

pub fn typecheck_stmt(
    local_env: &mut TypeEnvStack,
    global_env: &mut TypeEnvStack,
    stmt: &Stmt,
) -> Option<Type> {
    match &stmt {
        Stmt::Assignment(assign) => {
            let names = assign.variables().iter().map(|var| {
                if let Var::Name(tknref) = var {
                    if let TokenType::Identifier { identifier } = tknref.token_type() {
                        identifier.to_string()
                    } else {
                        panic!("Invalid token type")
                    }
                } else {
                    panic!("Invalid variables");
                }
            });
            let expressions = assign.expressions().iter();
            for (name, expr) in names.zip(expressions) {
                let ty = typecheck_expr(local_env, global_env, expr);
                global_env.bind(name, ty.unwrap());
            }
            None
        }
        Stmt::LocalAssignment(local_assign) => {
            let names = local_assign.names().iter().map(|tknref| {
                if let TokenType::Identifier { identifier } = tknref.token_type() {
                    identifier.to_string()
                } else {
                    panic!("Invalid token type")
                }
            });
            let expressions = local_assign.expressions().iter();
            for (name, expr) in names.zip(expressions) {
                let ty = typecheck_expr(local_env, global_env, expr);
                local_env.bind(name, ty.unwrap());
            }
            None
        }
        Stmt::FunctionDeclaration(func_dec) => {
            local_env.push_env();
            // only global funciton, not working class method and children function on table
            let func_name = func_dec.name().names().first().unwrap().to_string();
            let params: Vec<Param> = func_dec
                .body()
                .parameters()
                .iter()
                .map(|param| {
                    if let Parameter::Name(tknref) = param {
                        if let TokenType::Identifier { identifier } = tknref.token_type() {
                            Param {
                                symbol: identifier.to_string(),
                                ty: Type::Unknown,
                            }
                        } else {
                            panic!("Invalid token type")
                        }
                    } else {
                        panic!("Invalid paramters")
                    }
                })
                .collect();
            for param in params.iter() {
                local_env.bind(param.symbol.clone(), param.ty.clone());
            }
            let ret_ty = typecheck_block(local_env, global_env, func_dec.body().block());
            let func_ty = Type::Function(FunctionType {
                params,
                ret_ty: Box::new(ret_ty),
            });
            global_env.bind(func_name, func_ty);
            local_env.pop_env();
            None
        }
        Stmt::LocalFunction(local_func_dec) => {
            local_env.push_env();
            let func_name = local_func_dec.name().to_string();
            let params: Vec<Param> = local_func_dec
                .body()
                .parameters()
                .iter()
                .map(|param| {
                if let Parameter::Name(tknref) = param {
                    if let TokenType::Identifier { identifier } = tknref.token_type() {
                        Param {
                            symbol: identifier.to_string(),
                            ty: Type::Unknown,
                        }
                    } else {
                        panic!("Invalid token type")
                    }
                } else {
                    panic!("Invalid paramters")
                }
            }).collect();
            for param in params.iter() {
                local_env.bind(param.symbol.clone(), param.ty.clone());
            }
            let ret_ty = typecheck_block(local_env, global_env, local_func_dec.body().block());
            let func_ty = Type::Function(FunctionType {
                params,
                ret_ty: Box::new(ret_ty),
            });
            local_env.pop_env();
            local_env.bind(func_name, func_ty);
            None
        }
        _ => panic!("not inplementend"),
    }
}

pub fn typecheck_last_stmt(
    local_env: &mut TypeEnvStack,
    global_env: &mut TypeEnvStack,
    last_stmt: &LastStmt,
) -> Option<Type> {
    match &last_stmt {
        LastStmt::Return(ret) => {
            let ret_types: Vec<Type> = ret
                .returns()
                .iter()
                .map(|expr| {
                    typecheck_expr(local_env, global_env, expr)
                        .unwrap_or_else(|e| panic!("Invalid return expression: {e}"))
                })
                .collect();
            match ret_types.len() {
                0 => Some(Type::None),
                1 => Some(ret_types.first().unwrap().clone()),
                _ => Some(Type::MultiValue(MultiValueType {
                    types: ret_types.clone(),
                })),
            }
        }
        LastStmt::Break(_) => Some(Type::None),
        _ => unimplemented!("unimplemented last statement"),
    }
}

pub fn typecheck_expr(
    local_env: &mut TypeEnvStack,
    global_env: &mut TypeEnvStack,
    expr: &Expression,
) -> Result<Type> {
    match &expr {
        Expression::Number(tknref) => match &tknref.token_type() {
            TokenType::Number { .. } => Ok(Type::Number),
            _ => {
                let token_type = tknref.token_type();
                let err_string = format!("Expected Number Type. But actually got {token_type:?}");
                Err(anyhow!(err_string))
            }
        },
        Expression::String(tknref) => match &tknref.token_type() {
            TokenType::StringLiteral { .. } => Ok(Type::String),
            _ => {
                let token_type = tknref.token_type();
                let err_string = format!("Expected String Type. But actually got {token_type:?}");
                Err(anyhow!(err_string))
            }
        },
        Expression::Symbol(tknref) => match &tknref.token_type() {
            TokenType::Symbol { symbol } => match symbol {
                Symbol::True => Ok(Type::Boolean),
                Symbol::False => Ok(Type::Boolean),
                _ => {
                    let token_type = tknref.token_type();
                    let err_string =
                        format!("Expected Symbol Type. But actually got {token_type:?}");
                    Err(anyhow!(err_string))
                }
            },
            _ => {
                let token_type = tknref.token_type();
                let err_string = format!("Expected Symbol Type. But actually got {token_type:?}");
                Err(anyhow!(err_string))
            }
        },
        Expression::TableConstructor(_) => Ok(Type::Table),
        Expression::BinaryOperator { lhs, binop, rhs } => {
            let lhs_ty = typecheck_expr(local_env, global_env, lhs.as_ref())?;
            let rhs_ty = typecheck_expr(local_env, global_env, rhs.as_ref())?;
            match binop {
                BinOp::Plus(_) | BinOp::Minus(_) => {
                    if lhs_ty == rhs_ty {
                        match lhs_ty {
                            Type::Number => Ok(Type::Number),
                            _ => {
                                let err_string =
                                    format!("Expected Arithmetic type. Got {lhs_ty:?}");
                                Err(anyhow!(err_string))
                            }
                        }
                    } else {
                        let err_string = format!(
                            "Different type, Got left is {lhs_ty:?}, right is {rhs_ty:?}.",
                        );
                        Err(anyhow!(err_string))
                    }
                }
                _ => Err(anyhow!("Not unimplemented")),
            }
        }
        Expression::UnaryOperator { unop, expression } => {
            let ty = typecheck_expr(local_env, global_env, expression)?;
            match unop {
                UnOp::Minus(_) => match ty {
                    Type::Number => Ok(Type::Number),
                    _ => Err(anyhow!("Expected Number for 'Minus' unary operator")),
                },
                UnOp::Not(_) => match ty {
                    Type::Boolean => Ok(Type::Boolean),
                    _ => Err(anyhow!("Expected Boolean for 'Not' unary operator")),
                },
                UnOp::Hash(_) => match ty {
                    Type::Table => Ok(Type::Number),
                    Type::String => Ok(Type::Number),
                    _ => Err(anyhow!(
                        "Expected Table or String for 'Hash' unary operator"
                    )),
                },
                // UnOp::Tilde(_) => {}
                _ => Err(anyhow!("Not unimplemtend")),
            }
        }
        Expression::Var(var) => match var {
            Var::Expression(_) => Err(anyhow!("unimplemented")),
            Var::Name(tknref) => {
                let name = if let TokenType::Identifier { identifier } = tknref.token_type() {
                    identifier.to_string()
                } else {
                    panic!("Invalid token type")
                };
                if let Some(local_ty) = local_env.lookup(&name) {
                    Ok(local_ty)
                } else if let Some(global_ty) = global_env.lookup(&name) {
                    Ok(global_ty)
                } else {
                    Err(anyhow!("Not found type"))
                }
            }
            _ => panic!("panic"),
        },
        Expression::Function(anonymous_func) => {
            local_env.push_env();
            let params: Vec<Param> = anonymous_func
                .body()
                .parameters()
                .iter()
                .map(|param| {
                    if let Parameter::Name(tknref) = param {
                        if let TokenType::Identifier { identifier } = tknref.token_type() {
                            Param {
                                symbol: identifier.to_string(),
                                ty: Type::Unknown,
                            }
                        } else {
                            panic!("Invalid token type")
                        }
                    } else {
                        panic!("Invalid paramters")
                    }
                })
                .collect();
            for param in params.iter() {
                local_env.bind(param.symbol.clone(), param.ty.clone());
            }
            let ret_ty = typecheck_block(local_env, global_env, anonymous_func.body().block());
            let func_ty = Type::Function(FunctionType { params, ret_ty: Box::new(ret_ty) });
            local_env.pop_env();
            Ok(func_ty)
        },
        _ => Err(anyhow!("Not unimplemtend: got {:#?}", expr)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use full_moon::{
        ShortString,
        ast::{BinOp, Expression, TableConstructor},
        tokenizer::{StringLiteralQuoteType, Symbol, Token, TokenReference, TokenType},
    };

    #[test]
    fn test_number() {
        let mut local_env = TypeEnvStack::new();
        let mut global_env = TypeEnvStack::new();
        let expr = Expression::Number(TokenReference::new(
            vec![],
            Token::new(TokenType::Number {
                text: ShortString::new("1"),
            }),
            vec![],
        ));
        assert_eq!(
            typecheck_expr(&mut local_env, &mut global_env, &expr).unwrap(),
            Type::Number
        );
        let expr = Expression::Number(TokenReference::new(
            vec![],
            Token::new(TokenType::Number {
                text: ShortString::new("1"),
            }),
            vec![],
        ));
        assert_eq!(
            typecheck_expr(&mut local_env, &mut global_env, &expr).unwrap(),
            Type::Number
        );
    }

    #[test]
    fn test_binaryop() {
        let mut local_env = TypeEnvStack::new();
        let mut global_env = TypeEnvStack::new();
        // normal-test: Number + Number => Number
        let expr = Expression::BinaryOperator {
            lhs: Box::new(Expression::Number(TokenReference::new(
                vec![],
                Token::new(TokenType::Number {
                    text: ShortString::new("1"),
                }),
                vec![],
            ))),
            binop: BinOp::Plus(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::Plus,
                }),
                vec![],
            )),
            rhs: Box::new(Expression::Number(TokenReference::new(
                vec![],
                Token::new(TokenType::Number {
                    text: ShortString::new("2"),
                }),
                vec![],
            ))),
        };
        assert_eq!(
            typecheck_expr(&mut local_env, &mut global_env, &expr).unwrap(),
            Type::Number
        );

        // error-test: Number + Boolean
        let expr = Expression::BinaryOperator {
            lhs: Box::new(Expression::Number(TokenReference::new(
                vec![],
                Token::new(TokenType::Number {
                    text: ShortString::new("1"),
                }),
                vec![],
            ))),
            binop: BinOp::Plus(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::Plus,
                }),
                vec![],
            )),
            rhs: Box::new(Expression::Symbol(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::True,
                }),
                vec![],
            ))),
        };
        assert_eq!(
            typecheck_expr(&mut local_env, &mut global_env, &expr)
                .unwrap_err()
                .to_string(),
            "Different type, Got left is Number, right is Boolean.".to_string()
        );

        // error-test: Boolean + Number
        let expr = Expression::BinaryOperator {
            lhs: Box::new(Expression::Symbol(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::True,
                }),
                vec![],
            ))),
            binop: BinOp::Plus(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::Plus,
                }),
                vec![],
            )),
            rhs: Box::new(Expression::Number(TokenReference::new(
                vec![],
                Token::new(TokenType::Number {
                    text: ShortString::new("1"),
                }),
                vec![],
            ))),
        };
        assert_eq!(
            typecheck_expr(&mut local_env, &mut global_env, &expr)
                .unwrap_err()
                .to_string(),
            "Different type, Got left is Boolean, right is Number.".to_string()
        );

        // error-test: Boolean + Boolean
        let expr = Expression::BinaryOperator {
            lhs: Box::new(Expression::Symbol(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::True,
                }),
                vec![],
            ))),
            binop: BinOp::Plus(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::Plus,
                }),
                vec![],
            )),
            rhs: Box::new(Expression::Symbol(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::True,
                }),
                vec![],
            ))),
        };
        assert_eq!(
            typecheck_expr(&mut local_env, &mut global_env, &expr)
                .unwrap_err()
                .to_string(),
            "Expected Arithmetic type. Got Boolean".to_string()
        );
    }

    #[test]
    fn test_unaryop() {
        let mut local_env = TypeEnvStack::new();
        let mut global_env = TypeEnvStack::new();
        // Minus Operator
        // normal-test: '-' + Number
        let expr = Expression::UnaryOperator {
            unop: UnOp::Minus(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::Minus,
                }),
                vec![],
            )),
            expression: Box::new(Expression::Number(TokenReference::new(
                vec![],
                Token::new(TokenType::Number {
                    text: ShortString::new("12"),
                }),
                vec![],
            ))),
        };
        assert_eq!(
            typecheck_expr(&mut local_env, &mut global_env, &expr).unwrap(),
            Type::Number
        );
        // error-test: '-' + NotNumber
        let expr = Expression::UnaryOperator {
            unop: UnOp::Minus(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::Minus,
                }),
                vec![],
            )),
            expression: Box::new(Expression::Symbol(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::True,
                }),
                vec![],
            ))),
        };
        assert_eq!(
            typecheck_expr(&mut local_env, &mut global_env, &expr)
                .unwrap_err()
                .to_string(),
            "Expected Number for 'Minus' unary operator".to_string(),
        );

        // Not Operator
        // normal-test: 'not' + Boolean
        let expr = Expression::UnaryOperator {
            unop: UnOp::Not(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::Not,
                }),
                vec![],
            )),
            expression: Box::new(Expression::Symbol(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::True,
                }),
                vec![],
            ))),
        };
        assert_eq!(
            typecheck_expr(&mut local_env, &mut global_env, &expr).unwrap(),
            Type::Boolean
        );

        // error-test: 'not' + NotBoolean
        let expr = Expression::UnaryOperator {
            unop: UnOp::Not(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::Not,
                }),
                vec![],
            )),
            expression: Box::new(Expression::Number(TokenReference::new(
                vec![],
                Token::new(TokenType::Number {
                    text: ShortString::new("12"),
                }),
                vec![],
            ))),
        };
        assert_eq!(
            typecheck_expr(&mut local_env, &mut global_env, &expr)
                .unwrap_err()
                .to_string(),
            "Expected Boolean for 'Not' unary operator".to_string(),
        );

        // Hash Operator
        // normal-test: '#' + Table
        let expr = Expression::UnaryOperator {
            unop: UnOp::Hash(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::Hash,
                }),
                vec![],
            )),
            expression: Box::new(Expression::TableConstructor(TableConstructor::new())),
        };
        assert_eq!(
            typecheck_expr(&mut local_env, &mut global_env, &expr).unwrap(),
            Type::Number
        );

        // normal-test: '#' + String
        let expr = Expression::UnaryOperator {
            unop: UnOp::Hash(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::Hash,
                }),
                vec![],
            )),
            expression: Box::new(Expression::String(TokenReference::new(
                vec![],
                Token::new(TokenType::StringLiteral {
                    literal: ShortString::new("hello"),
                    multi_line_depth: 0,
                    quote_type: StringLiteralQuoteType::Single,
                }),
                vec![],
            ))),
        };
        assert_eq!(
            typecheck_expr(&mut local_env, &mut global_env, &expr).unwrap(),
            Type::Number,
        );

        // normal-test: '#' + Boolean
        let expr = Expression::UnaryOperator {
            unop: UnOp::Hash(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::Hash,
                }),
                vec![],
            )),
            expression: Box::new(Expression::Symbol(TokenReference::new(
                vec![],
                Token::new(TokenType::Symbol {
                    symbol: Symbol::True,
                }),
                vec![],
            ))),
        };
        assert_eq!(
            typecheck_expr(&mut local_env, &mut global_env, &expr)
                .unwrap_err()
                .to_string(),
            "Expected Table or String for 'Hash' unary operator".to_string(),
        );
    }
    #[test]
    fn test_type_env_stack() {
        let mut env = TypeEnvStack::new();
        env.bind("a".to_string(), Type::Number);
        assert_eq!(env.lookup("a"), Some(Type::Number));
    }

    #[test]
    fn test_global_assign() {
        let mut local_env = TypeEnvStack::new();
        let mut global_env = TypeEnvStack::new();
        let ast = full_moon::parse(
            r#"
        a = 1
        b = "Hello"
        "#,
        )
        .unwrap();
        for stmt in ast.nodes().stmts() {
            typecheck_stmt(&mut local_env, &mut global_env, stmt);
        }
        assert_eq!(global_env.lookup("a"), Some(Type::Number));
        assert_eq!(global_env.lookup("b"), Some(Type::String));
    }
    #[test]
    fn test_local_assign() {
        let mut local_env = TypeEnvStack::new();
        let mut global_env = TypeEnvStack::new();
        let ast = full_moon::parse(
            r#"
        local a = 1
        local b = "Hello"
        "#,
        )
        .unwrap();
        for stmt in ast.nodes().stmts() {
            typecheck_stmt(&mut local_env, &mut global_env, stmt);
        }
        assert_eq!(local_env.lookup("a"), Some(Type::Number));
        assert_eq!(local_env.lookup("b"), Some(Type::String));
    }
    #[test]
    fn test_global_function_declaration() {
        let mut local_env = TypeEnvStack::new();
        let mut global_env = TypeEnvStack::new();
        let ast = full_moon::parse(
            r#"
        function minmax(x, y)
            local a = 12
            local b = 13
            return x, y
        end
        "#,
        )
        .unwrap();
        typecheck_block(&mut local_env, &mut global_env, ast.nodes());
        assert_eq!(
            global_env.lookup("minmax"),
            Some(Type::Function(FunctionType {
                params: vec![
                    Param {
                        symbol: "x".to_string(),
                        ty: Type::Unknown
                    },
                    Param {
                        symbol: "y".to_string(),
                        ty: Type::Unknown
                    }
                ],
                ret_ty: Box::new(Some(Type::MultiValue(MultiValueType {
                    types: vec![Type::Unknown, Type::Unknown]
                })))
            }))
        );
        assert_eq!(local_env.lookup("x"), None);
        assert_eq!(local_env.lookup("y"), None);
        assert_eq!(local_env.lookup("a"), None);
        assert_eq!(local_env.lookup("b"), None);
    }
    #[test]
    fn test_local_function_declaration() {
        let mut local_env = TypeEnvStack::new();
        let mut global_env = TypeEnvStack::new();
        let ast = full_moon::parse(
            r#"
        local function minmax(x, y)
            local a = 12
            local b = 13
            return x, y
        end
        "#,
        )
        .unwrap();
        typecheck_block(&mut local_env, &mut global_env, ast.nodes());
        assert_eq!(
            local_env.lookup("minmax"),
            Some(Type::Function(FunctionType {
                params: vec![
                    Param {
                        symbol: "x".to_string(),
                        ty: Type::Unknown
                    },
                    Param {
                        symbol: "y".to_string(),
                        ty: Type::Unknown
                    }
                ],
                ret_ty: Box::new(Some(Type::MultiValue(MultiValueType {
                    types: vec![Type::Unknown, Type::Unknown]
                })))
            }))
        );
        assert_eq!(local_env.lookup("x"), None);
        assert_eq!(local_env.lookup("y"), None);
        assert_eq!(local_env.lookup("a"), None);
        assert_eq!(local_env.lookup("b"), None);
    }
    #[test]
    fn test_local_anonymous_function_assignment() {
        let mut local_env = TypeEnvStack::new();
        let mut global_env = TypeEnvStack::new();
        let ast = full_moon::parse(
            r#"
        local minmax = function(x, y)
            local a = 12
            local b = 13
            return x, y
        end
        "#,
        )
        .unwrap();
        typecheck_block(&mut local_env, &mut global_env, ast.nodes());
        assert_eq!(
            local_env.lookup("minmax"),
            Some(Type::Function(FunctionType {
                params: vec![
                    Param {
                        symbol: "x".to_string(),
                        ty: Type::Unknown
                    },
                    Param {
                        symbol: "y".to_string(),
                        ty: Type::Unknown
                    }
                ],
                ret_ty: Box::new(Some(Type::MultiValue(MultiValueType {
                    types: vec![Type::Unknown, Type::Unknown]
                })))
            }))
        );
        assert_eq!(local_env.lookup("x"), None);
        assert_eq!(local_env.lookup("y"), None);
        assert_eq!(local_env.lookup("a"), None);
        assert_eq!(local_env.lookup("b"), None);
    }
}
