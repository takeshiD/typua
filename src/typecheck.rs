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

pub fn typecheck(expr: &full_moon::ast::Expression) -> Result<Type, String> {
    use full_moon::ast::{BinOp, Expression};
    use full_moon::tokenizer::{Symbol, TokenType};
    match &expr {
        Expression::Number(tknref) => match &tknref.token_type() {
            TokenType::Number { text } => Ok(Type::Number),
            _ => {
                let token_type = tknref.token_type();
                let err_string = format!("Expected Number Type. But actually got {:?}", token_type);
                Err(err_string)
            }
        },
        Expression::String(tknref) => match &tknref.token_type() {
            TokenType::StringLiteral {
                literal,
                multi_line_depth,
                quote_type,
            } => Ok(Type::String),
            _ => {
                let token_type = tknref.token_type();
                let err_string = format!("Expected String Type. But actually got {:?}", token_type);
                Err(err_string)
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
                    Err(err_string)
                }
            },
            _ => {
                let token_type = tknref.token_type();
                let err_string = format!("Expected Symbol Type. But actually got {:?}", token_type);
                Err(err_string)
            }
        },
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
                                Err(err_string)
                            }
                        }
                    } else {
                        let err_string = format!(
                            "Different type, Got left is {:?}, right is {:?}.",
                            lhs_ty, rhs_ty
                        );
                        Err(err_string)
                    }
                }
                _ => Err("Not unimplemented".to_string()),
            }
        }
        // Expression::Parentheses {
        //     contained,
        //     expression,
        // } => {}
        // Expression::UnaryOperator { unop, expression } => {}
        _ => unimplemented!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use full_moon::ShortString;
    use full_moon::ast::{BinOp, Expression};
    use full_moon::tokenizer::{Symbol, Token, TokenReference, TokenType};

    #[test]
    fn test_number() {
        let expr1 = Expression::Number(TokenReference::new(
            vec![],
            Token::new(TokenType::Number {
                text: ShortString::new("1"),
            }),
            vec![],
        ));
        assert_eq!(typecheck(&expr1), Ok(Type::Number));
        let expr1 = Expression::Number(TokenReference::new(
            vec![],
            Token::new(TokenType::Number {
                text: ShortString::new("1"),
            }),
            vec![],
        ));
        assert_eq!(typecheck(&expr1), Ok(Type::Number));
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
        assert_eq!(typecheck(&expr1), Ok(Type::Number));

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
            typecheck(&expr2),
            Err("Different type, Got left is Number, right is Boolean.".to_string())
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
                    text: ShortString::new("1")
                }),
                vec![],
            ))),
        };
        assert_eq!(
            typecheck(&expr3),
            Err("Different type, Got left is Boolean, right is Number.".to_string())
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
            typecheck(&expr4),
            Err("Expected Arithmetic type. Got Boolean".to_string())
        );
    }
}
