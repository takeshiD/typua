use crate::{annotation, types::TypeKind};

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationInfo {
    pub content: String,
    pub tag: AnnotationTag,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationTag {
    Type(TypeKind),
    Alias,
    As,
    Class,
}

pub fn parse_annotation(content: String) -> Option<AnnotationInfo> {
    unimplemented!()
}

pub fn concat_tokens<'a>(tokens: impl Iterator<Item = &'a full_moon::tokenizer::Token>) -> String {
    let strings: Vec<String> = tokens.map(|t| t.to_string()).collect();
    strings.concat().trim().to_string()
}

#[cfg(test)]
mod concat_tokens {
    use super::*;
    use full_moon::ShortString;
    use full_moon::tokenizer::{Token, TokenType};
    use pretty_assertions::assert_eq;
    use unindent::unindent;
    #[test]
    fn singleline() {
        let tokens = vec![
            Token::new(TokenType::Whitespace {
                characters: ShortString::new("\n"),
            }),
            Token::new(TokenType::SingleLineComment {
                comment: ShortString::new("-@type Container"),
            }),
            Token::new(TokenType::Whitespace {
                characters: ShortString::new("\n"),
            }),
        ];
        let ann_string = concat_tokens(tokens.iter());
        assert_eq!(ann_string, unindent(r#"---@type Container"#));
    }
    #[test]
    fn multiline() {
        let tokens = vec![
            Token::new(TokenType::Whitespace {
                characters: ShortString::new("\n"),
            }),
            Token::new(TokenType::SingleLineComment {
                comment: ShortString::new("-@class Position2d"),
            }),
            Token::new(TokenType::Whitespace {
                characters: ShortString::new("\n"),
            }),
            Token::new(TokenType::SingleLineComment {
                comment: ShortString::new("-@field x number"),
            }),
            Token::new(TokenType::Whitespace {
                characters: ShortString::new("\n"),
            }),
            Token::new(TokenType::SingleLineComment {
                comment: ShortString::new("-@field y number"),
            }),
            Token::new(TokenType::Whitespace {
                characters: ShortString::new("\n"),
            }),
        ];
        let ann_string = concat_tokens(tokens.iter());
        assert_eq!(
            ann_string,
            unindent(
                r#"
            ---@class Position2d
            ---@field x number
            ---@field y number"#
            )
        );
    }
}

#[cfg(test)]
mod parse_annotation_normaltest {
    use super::*;
    use pretty_assertions::assert_eq;
    use unindent::unindent;
    #[test]
    fn type_annotation() {
        // sigle type
        let content = unindent(r#"---@type number"#);
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.is_some(), true);
        assert_eq!(
            ann_info.unwrap(),
            AnnotationInfo {
                content: unindent(r#"---@type number"#),
                tag: AnnotationTag::Type(TypeKind::Number),
            }
        );
        // optional
        let content = unindent(r#"---@type number?"#);
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.is_some(), true);
        assert_eq!(
            ann_info.unwrap(),
            AnnotationInfo {
                content: unindent(r#"---@type number?"#),
                tag: AnnotationTag::Type(TypeKind::Union(vec![TypeKind::Number, TypeKind::Nil])),
            }
        );
        // union
        let content = unindent(r#"---@type number|string"#);
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.is_some(), true);
        assert_eq!(
            ann_info.unwrap(),
            AnnotationInfo {
                content: unindent(r#"---@type number|string"#),
                tag: AnnotationTag::Type(TypeKind::Union(vec![TypeKind::Number, TypeKind::String])),
            }
        );
        // if no annotation, return any type
        let content = unindent(r#""#);
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.is_none(), true);
        assert_eq!(
            ann_info.unwrap(),
            AnnotationInfo {
                content: unindent(r#""#),
                tag: AnnotationTag::Type(TypeKind::Any),
            }
        );
        // array
        let content = unindent(r#"---@type string[]"#);
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.is_some(), true);
        assert_eq!(
            ann_info.unwrap(),
            AnnotationInfo {
                content: unindent(r#"---@type string[]"#),
                tag: AnnotationTag::Type(TypeKind::Array(Box::new(TypeKind::String))),
            }
        );
        // array
        let content = unindent(r#"---@type string[]"#);
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.is_some(), true);
        assert_eq!(
            ann_info.unwrap(),
            AnnotationInfo {
                content: unindent(r#"---@type string[]"#),
                tag: AnnotationTag::Type(TypeKind::Array(Box::new(TypeKind::String))),
            }
        );
        // dictionary
        let content = unindent(r#"---@type { [string]: boolean }"#);
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.is_some(), true);
        assert_eq!(
            ann_info.unwrap(),
            AnnotationInfo {
                content: unindent(r#"---@type { [string]: boolean }"#),
                tag: AnnotationTag::Type(TypeKind::Dict {
                    key: Box::new(TypeKind::String),
                    value: Box::new(TypeKind::Boolean),
                }),
            }
        );
        // table
        let content = unindent(r#"---@type table<string, number>"#);
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.is_some(), true);
        assert_eq!(
            ann_info.unwrap(),
            AnnotationInfo {
                content: unindent(r#"---@type table<string, number>"#),
                tag: AnnotationTag::Type(TypeKind::KVTable {
                    key: Box::new(TypeKind::String),
                    value: Box::new(TypeKind::Number),
                }),
            }
        );
    }
}
