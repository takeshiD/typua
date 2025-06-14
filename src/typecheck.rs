use anyhow::{Result, anyhow};
use full_moon::{
    ast::{BinOp, Expression, UnOp},
    tokenizer::{Symbol, TokenType},
};

#[derive(Debug, PartialEq)]
pub enum Type {
    Nil,
    Boolean,
    Number,
    String,
    Table,
    Function(FunctionType),
}

#[derive(Debug, PartialEq)]
pub struct Param {
    symbol: String,
    ty: Type,
}

#[derive(Debug, PartialEq)]
pub struct FunctionType {
    params: Vec<Param>,
    ret_ty: Box<Type>,
}

pub fn typecheck(expr: &full_moon::ast::Expression) -> Result<Type> {
    match &expr {
        Expression::Number(tknref) => match &tknref.token_type() {
            TokenType::Number { .. } => Ok(Type::Number),
            _ => {
                let token_type = tknref.token_type();
                let err_string = format!("Expected Number Type. But actually got {:?}", token_type);
                Err(anyhow!(err_string))
            }
        },
        Expression::String(tknref) => match &tknref.token_type() {
            TokenType::StringLiteral { .. } => Ok(Type::String),
            _ => {
                let token_type = tknref.token_type();
                let err_string = format!("Expected String Type. But actually got {:?}", token_type);
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
                        format!("Expected Symbol Type. But actually got {:?}", token_type);
                    Err(anyhow!(err_string))
                }
            },
            _ => {
                let token_type = tknref.token_type();
                let err_string = format!("Expected Symbol Type. But actually got {:?}", token_type);
                Err(anyhow!(err_string))
            }
        },
        Expression::TableConstructor(_) => Ok(Type::Table),
        Expression::BinaryOperator { lhs, binop, rhs } => {
            let lhs_ty = typecheck(lhs.as_ref())?;
            let rhs_ty = typecheck(rhs.as_ref())?;
            match binop {
                BinOp::Plus(_) | BinOp::Minus(_) => {
                    if lhs_ty == rhs_ty {
                        match lhs_ty {
                            Type::Number => Ok(Type::Number),
                            _ => {
                                let err_string =
                                    format!("Expected Arithmetic type. Got {:?}", lhs_ty);
                                Err(anyhow!(err_string))
                            }
                        }
                    } else {
                        let err_string = format!(
                            "Different type, Got left is {:?}, right is {:?}.",
                            lhs_ty, rhs_ty
                        );
                        Err(anyhow!(err_string))
                    }
                }
                _ => Err(anyhow!("Not unimplemented")),
            }
        }
        Expression::UnaryOperator { unop, expression } => {
            let ty = typecheck(expression)?;
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
        let expr1 = Expression::Number(TokenReference::new(
            vec![],
            Token::new(TokenType::Number {
                text: ShortString::new("1"),
            }),
            vec![],
        ));
        assert_eq!(typecheck(&expr1).unwrap(), Type::Number);
        let expr1 = Expression::Number(TokenReference::new(
            vec![],
            Token::new(TokenType::Number {
                text: ShortString::new("1"),
            }),
            vec![],
        ));
        assert_eq!(typecheck(&expr1).unwrap(), Type::Number);
    }

    #[test]
    fn test_binaryop() {
        // normal-test: Number + Number => Number
        let expr1 = Expression::BinaryOperator {
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
        assert_eq!(typecheck(&expr1).unwrap(), Type::Number);

        // error-test: Number + Boolean
        let expr2 = Expression::BinaryOperator {
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
            typecheck(&expr2).unwrap_err().to_string(),
            "Different type, Got left is Number, right is Boolean.".to_string()
        );

        // error-test: Boolean + Number
        let expr3 = Expression::BinaryOperator {
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
            typecheck(&expr3).unwrap_err().to_string(),
            "Different type, Got left is Boolean, right is Number.".to_string()
        );

        // error-test: Boolean + Boolean
        let expr4 = Expression::BinaryOperator {
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
            typecheck(&expr4).unwrap_err().to_string(),
            "Expected Arithmetic type. Got Boolean".to_string()
        );
    }

    #[test]
    fn test_unaryop() {
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
        assert_eq!(typecheck(&expr).unwrap(), Type::Number);
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
            typecheck(&expr).unwrap_err().to_string(),
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
        assert_eq!(typecheck(&expr).unwrap(), Type::Boolean);

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
            typecheck(&expr).unwrap_err().to_string(),
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
        assert_eq!(typecheck(&expr).unwrap(), Type::Number);

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
        assert_eq!(typecheck(&expr).unwrap(), Type::Number,);

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
            typecheck(&expr).unwrap_err().to_string(),
            "Expected Table or String for 'Hash' unary operator".to_string(),
        );
    }
}
