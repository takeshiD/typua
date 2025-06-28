use anyhow::{Result, anyhow};
use full_moon::{
    ast::{Assignment, BinOp, Block, Expression, Stmt, UnOp, Var},
    tokenizer::{Symbol, TokenType},
};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    None,
    Any,
    Nil,
    Boolean,
    Number,
    String,
    Table,
    // Function(FunctionType),
}

#[derive(Debug, PartialEq)]
pub struct Param {
    symbol: String,
    ty: Type,
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
        self.depth -= 1;
        self.envs.remove(&self.depth);
    }
    pub fn bind(&mut self, name: String, type_: Type) {
        println!("Env: {:#?}", self.envs);
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
    pub fn lookup(&mut self, name: &str) -> Option<Type> {
        for (_, env) in self.envs.iter_mut().rev() {
            if let Some(ty) = env.get_mut(name) {
                return ty.clone();
            }
        }
        None
    }
    pub fn is_empty(&self) -> bool {
        self.envs.is_empty()
    }
}

#[derive(Debug, PartialEq)]
pub struct FunctionType {
    params: Vec<Param>,
    ret_ty: Box<Type>,
}

pub fn typecheck_block(
    local_env: &mut TypeEnvStack,
    global_env: &mut TypeEnvStack,
    block: &Block,
) -> Result<Type> {
    for stmt in block.stmts() {
        let stmt_type = typecheck_stmt(local_env, global_env, stmt)?;
    }
    Ok(Type::None)
}

pub fn typecheck_stmt(
    local_env: &mut TypeEnvStack,
    global_env: &mut TypeEnvStack,
    stmt: &Stmt,
) -> Result<Type> {
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
                println!("{name} = {ty:?}");
                global_env.bind(name, ty.unwrap());
            }
            Ok(Type::None)
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
                println!("local '{name}' = {ty:?}");
                // local_env.bind(name, typecheck_expr(local_env, global_env, expr)?);
                local_env.bind(name, ty.unwrap());
            }
            println!(
                "depth: {}, len: {}, lookup: {:?}",
                local_env.depth.clone(),
                local_env.envs.len(),
                local_env.lookup("a")
            );
            Ok(Type::None)
        }
        _ => panic!("not inplementend"),
    }
}

pub fn typecheck_expr(
    _local_env: &TypeEnvStack,
    _global_env: &TypeEnvStack,
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
            let lhs_ty = typecheck_expr(_local_env, _global_env, lhs.as_ref())?;
            let rhs_ty = typecheck_expr(_local_env, _global_env, rhs.as_ref())?;
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
            let ty = typecheck_expr(_local_env, _global_env, expression)?;
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
        let local_env = TypeEnvStack::new();
        let global_env = TypeEnvStack::new();
        let expr = Expression::Number(TokenReference::new(
            vec![],
            Token::new(TokenType::Number {
                text: ShortString::new("1"),
            }),
            vec![],
        ));
        assert_eq!(
            typecheck_expr(&local_env, &global_env, &expr).unwrap(),
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
            typecheck_expr(&local_env, &global_env, &expr).unwrap(),
            Type::Number
        );
    }

    #[test]
    fn test_binaryop() {
        let local_env = TypeEnvStack::new();
        let global_env = TypeEnvStack::new();
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
            typecheck_expr(&local_env, &global_env, &expr).unwrap(),
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
            typecheck_expr(&local_env, &global_env, &expr)
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
            typecheck_expr(&local_env, &global_env, &expr)
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
            typecheck_expr(&local_env, &global_env, &expr)
                .unwrap_err()
                .to_string(),
            "Expected Arithmetic type. Got Boolean".to_string()
        );
    }

    #[test]
    fn test_unaryop() {
        let local_env = TypeEnvStack::new();
        let global_env = TypeEnvStack::new();
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
            typecheck_expr(&local_env, &global_env, &expr).unwrap(),
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
            typecheck_expr(&local_env, &global_env, &expr)
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
            typecheck_expr(&local_env, &global_env, &expr).unwrap(),
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
            typecheck_expr(&local_env, &global_env, &expr)
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
            typecheck_expr(&local_env, &global_env, &expr).unwrap(),
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
            typecheck_expr(&local_env, &global_env, &expr).unwrap(),
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
            typecheck_expr(&local_env, &global_env, &expr)
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
            typecheck_stmt(&mut local_env, &mut global_env, stmt).unwrap();
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
            typecheck_stmt(&mut local_env, &mut global_env, stmt).unwrap();
        }
        assert_eq!(local_env.lookup("a"), Some(Type::Number));
        assert_eq!(local_env.lookup("b"), Some(Type::String));
    }

}
