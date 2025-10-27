use crate::span::Position;
use crate::types::TypeKind;
use crate::{error::TypuaError, span::Span};

use full_moon::tokenizer::Token;
use nom::sequence::terminated;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, multispace0, multispace1},
    combinator::map,
    error::ParseError,
    multi::separated_list1,
    sequence::{delimited, separated_pair},
};
use nom_locate::LocatedSpan;

type AnnotationSpan<'a> = LocatedSpan<&'a str>;

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationInfo {
    pub tag: AnnotationTag,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationTag {
    Type(TypeKind),
    Alias,
    As,
    Class,
}

/// helper function for parsing
pub fn concat_tokens<'a>(tokens: impl Iterator<Item = &'a full_moon::tokenizer::Token>) -> String {
    let strings: Vec<String> = tokens.map(|t| t.to_string()).collect();
    strings.concat().trim().to_string()
}

/// entry point for annotation parsing
pub fn parse_annotation(content: String) -> Vec<AnnotationInfo> {
    let span = AnnotationSpan::new(&content);
    if let Ok((_, tys)) = parse_type_annotation(span) {
        tys.iter()
            .map(|ty| AnnotationInfo {
                tag: AnnotationTag::Type(ty.clone()),
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// parsing type annotation
fn parse_type_annotation(i: AnnotationSpan) -> IResult<AnnotationSpan, Vec<TypeKind>> {
    let (i, _) = tag("---@type").parse(i)?;
    let (i, _) = multispace1.parse(i)?;
    separated_list1(ws(tag(",")), parse_type).parse(i)
}

/// parsing basictype number, string, boolean, any, nil
fn parse_type(i: AnnotationSpan) -> IResult<AnnotationSpan, TypeKind> {
    alt((
        parse_dict,
        parse_tabletype,
        parse_optional,
        parse_array,
        parse_union,
        parse_basictype,
    ))
    .parse(i)
}

fn parse_basictype(i: AnnotationSpan) -> IResult<AnnotationSpan, TypeKind> {
    alt((
        map(ws(tag("number")), |_| TypeKind::Number),
        map(ws(tag("boolean")), |_| TypeKind::Boolean),
        map(ws(tag("string")), |_| TypeKind::String),
        map(ws(tag("nil")), |_| TypeKind::Nil),
        map(ws(tag("any")), |_| TypeKind::Any),
    ))
    .parse(i)
}

fn parse_optional(i: AnnotationSpan) -> IResult<AnnotationSpan, TypeKind> {
    let (i, ty) = terminated(parse_basictype, tag("?")).parse(i)?;
    Ok((i, TypeKind::Union(vec![ty, TypeKind::Nil])))
}

fn parse_union(i: AnnotationSpan) -> IResult<AnnotationSpan, TypeKind> {
    let (i, tys) = separated_list1(ws(tag("|")), parse_basictype).parse(i)?;
    if tys.len() >= 2 {
        Ok((i, TypeKind::Union(tys)))
    } else {
        Err(nom::Err::Error(nom::error::Error::new(
            i,
            nom::error::ErrorKind::SeparatedList,
        )))
    }
}

fn parse_array(i: AnnotationSpan) -> IResult<AnnotationSpan, TypeKind> {
    let (i, ty) = terminated(parse_basictype, tag("[]")).parse(i)?;
    Ok((i, TypeKind::Array(Box::new(ty))))
}

fn parse_tabletype(i: AnnotationSpan) -> IResult<AnnotationSpan, TypeKind> {
    let (i, _) = tag("table").parse(i)?;
    let (i, (key_ty, val_ty)) = delimited(
        char('<'),
        separated_pair(parse_basictype, ws(char(',')), parse_basictype),
        char('>'),
    )
    .parse(i)?;
    Ok((
        i,
        TypeKind::KVTable {
            key: Box::new(key_ty),
            val: Box::new(val_ty),
        },
    ))
}

fn parse_dict(i: AnnotationSpan) -> IResult<AnnotationSpan, TypeKind> {
    let (i, (key_ty, val_ty)) = delimited(
        ws(char('{')),
        separated_pair(
            delimited(char('['), parse_basictype, char(']')),
            ws(char(':')),
            parse_basictype,
        ),
        ws(char('}')),
    )
    .parse(i)?;
    Ok((
        i,
        TypeKind::Dict {
            key: Box::new(key_ty),
            val: Box::new(val_ty),
        },
    ))
}

/// strip whitespace
fn ws<'a, O, E: ParseError<AnnotationSpan<'a>>, F>(
    inner: F,
) -> impl Parser<AnnotationSpan<'a>, Output = O, Error = E>
where
    F: Parser<AnnotationSpan<'a>, Output = O, Error = E>,
{
    delimited(multispace0, inner, multispace0)
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
mod parse_annotation_normal {
    use crate::span::Position;

    use super::*;
    use pretty_assertions::assert_eq;
    use unindent::unindent;
    #[test]
    fn type_annotation() {
        // sigle type
        let content = unindent(r#"---@type number"#);
        let ann_infos = parse_annotation(content);
        assert_eq!(ann_infos.len(), 1);
        assert_eq!(
            ann_infos[0],
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::Number),
                span: Span {
                    start: Position::new(1, 10),
                    end: Position::new(1, 15)
                }
            }
        );
        // multi assign
        let content = unindent(r#"---@type number,string"#);
        let ann_infos = parse_annotation(content);
        assert_eq!(ann_infos.len(), 2);
        assert_eq!(
            ann_infos[0],
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::Number),
                span: Span {
                    start: Position::new(1, 10),
                    end: Position::new(1, 15),
                }
            }
        );
        assert_eq!(
            ann_infos[1],
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::String),
                span: Span {
                    start: Position::new(1, 17),
                    end: Position::new(1, 22),
                }
            }
        );
        // optional
        let content = unindent(r#"---@type number?"#);
        let ann_infos = parse_annotation(content);
        assert_eq!(ann_infos.len(), 1);
        assert_eq!(
            ann_infos[0],
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::Union(vec![TypeKind::Number, TypeKind::Nil])),
                span: Span {
                    start: Position::new(1, 10),
                    end: Position::new(1, 16)
                }
            }
        );
        // union
        let content = unindent(r#"---@type number|string"#);
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.len(), 1);
        assert_eq!(
            ann_info[0],
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::Union(vec![TypeKind::Number, TypeKind::String])),
                span: Span {
                    start: Position::new(1, 10),
                    end: Position::new(1, 22)
                }
            }
        );
        // if no annotation, return any type
        let content = unindent(r#""#);
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.is_empty(), true);
        // array
        let content = unindent(r#"---@type string[]"#);
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.len(), 1);
        assert_eq!(
            ann_info[0],
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::Array(Box::new(TypeKind::String))),
                span: Span {
                    start: Position::new(1, 10),
                    end: Position::new(1, 17),
                }
            }
        );
        // dictionary
        let content = unindent(r#"---@type { [string]: boolean }"#);
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.len(), 1);
        assert_eq!(
            ann_info[0],
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::Dict {
                    key: Box::new(TypeKind::String),
                    val: Box::new(TypeKind::Boolean),
                }),
                span: Span {
                    start: Position::new(1, 10),
                    end: Position::new(1, 30),
                }
            }
        );
        // table
        let content = unindent(r#"---@type table<string, number>"#);
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.len(), 1);
        assert_eq!(
            ann_info[0],
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::KVTable {
                    key: Box::new(TypeKind::String),
                    val: Box::new(TypeKind::Number),
                }),
                span: Span {
                    start: Position::new(1, 10),
                    end: Position::new(1, 30),
                }
            }
        );
    }
}

#[cfg(test)]
mod parse_type_annotation_normal {
    use super::*;
    use pretty_assertions::assert_eq;
    #[test]
    fn basictype() {
        // sigle type
        let content = AnnotationSpan::new("---@type number");
        let result = parse_type_annotation(content);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap().1, vec![TypeKind::Number,]);
        // multi type
        let content = AnnotationSpan::new("---@type number , string ");
        let result = parse_type_annotation(content);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap().1, vec![TypeKind::Number, TypeKind::String,]);
        // optional
        let content = AnnotationSpan::new("---@type string?");
        let result = parse_type_annotation(content);
        assert_eq!(result.is_ok(), true);
        assert_eq!(
            result.unwrap().1,
            vec![TypeKind::Union(vec![TypeKind::String, TypeKind::Nil])]
        );
        // union
        let content = AnnotationSpan::new("---@type number|string");
        let result = parse_type_annotation(content);
        assert_eq!(result.is_ok(), true);
        assert_eq!(
            result.unwrap().1,
            vec![TypeKind::Union(vec![TypeKind::Number, TypeKind::String])]
        );
        // array
        let content = AnnotationSpan::new("---@type number[]");
        let result = parse_type_annotation(content);
        assert_eq!(result.is_ok(), true);
        assert_eq!(
            result.unwrap().1,
            vec![TypeKind::Array(Box::new(TypeKind::Number))]
        );
        // table
        let content = AnnotationSpan::new("---@type table<string, number>");
        let result = parse_type_annotation(content);
        assert_eq!(result.is_ok(), true);
        assert_eq!(
            result.unwrap().1,
            vec![TypeKind::KVTable {
                key: Box::new(TypeKind::String),
                val: Box::new(TypeKind::Number)
            }]
        );
        // dict
        let content = AnnotationSpan::new("---@type {[string]: boolean}");
        let result = parse_type_annotation(content);
        assert_eq!(result.is_ok(), true);
        assert_eq!(
            result.unwrap().1,
            vec![TypeKind::Dict {
                key: Box::new(TypeKind::String),
                val: Box::new(TypeKind::Boolean)
            }]
        );
    }
}
