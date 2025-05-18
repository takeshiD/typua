#[derive(Debug, PartialEq)]
pub enum Type {
    Nil,
    Boolean,
    Number,
    String,
    Table,
    Function(FunctionType),
    Integer,
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
    use full_moon::ast::Expression;
    use full_moon::tokenizer::TokenType;
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
            TokenType::Symbol { symbol } => Ok(Type::String),
            _ => {
                let token_type = tknref.token_type();
                let err_string = format!("Expected Symbol Type. But actually got {:?}", token_type);
                Err(err_string)
            }
        },
        _ => panic!("Not Implemented"), // Expression::BinaryOperator { lhs, binop, rhs } => {},
                                        // Expression::Parentheses { contained, expression } => {}
                                        // Expression::UnaryOperator { unop, expression } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use full_moon::ShortString;
    use full_moon::ast::Expression;
    use full_moon::tokenizer::{Token, TokenReference, TokenType};
    #[test]
    fn test_number() {
        let expr = Expression::Number(TokenReference::new(
            vec![],
            Token::new(TokenType::Number {
                text: ShortString::new("1"),
            }),
            vec![],
        ));
        assert_eq!(typecheck(&expr), Ok(Type::Number));
    }
}
