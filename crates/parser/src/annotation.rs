use typua_span::{Position, Span};
use typua_ty::{BoolLiteral, TypeKind};

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
use tracing::{debug, warn};

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
pub fn parse_annotation(content: &str) -> Vec<AnnotationInfo> {
    let span = AnnotationSpan::new(content);    debug!("parsing content length = {}", content.len());
    match parse_type_annotation(span) {
        Ok((span, infos)) => {
            debug!("annotation parse result: {:#?}", infos);
            debug!("annotation parse rest span: {:#?}", span);
            infos
        }
        Err(e) => {
            warn!("annotation parse to failed: {e}");
            Vec::new()
        }
    }
}

/// parsing type annotation
fn parse_type_annotation(i: AnnotationSpan) -> IResult<AnnotationSpan, Vec<AnnotationInfo>> {
    let (i, _) = tag("---@type").parse(i)?;
    let (i, _) = multispace1.parse(i)?;
    separated_list1(ws(tag(",")), parse_type).parse(i)
}

/// parsing basictype number, string, boolean, any, nil
fn parse_type(i: AnnotationSpan) -> IResult<AnnotationSpan, AnnotationInfo> {
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

fn parse_basictype(start_span: AnnotationSpan) -> IResult<AnnotationSpan, AnnotationInfo> {
    let (end_span, ty) = alt((
        map(ws(tag("number")), |_| TypeKind::Number),
        map(ws(tag("boolean")), |_| TypeKind::Boolean(BoolLiteral::Any)),
        map(ws(tag("string")), |_| TypeKind::String),
        map(ws(tag("nil")), |_| TypeKind::Nil),
        map(ws(tag("any")), |_| TypeKind::Any),
    ))
    .parse(start_span)?;
    let satrt_position = Position::new(start_span.location_line(), start_span.get_column() as u32);
    let end_position = Position::new(end_span.location_line(), end_span.get_column() as u32);
    Ok((
        end_span,
        AnnotationInfo {
            tag: AnnotationTag::Type(ty),
            span: Span {
                start: satrt_position,
                end: end_position,
            },
        },
    ))
}

fn parse_optional(start_span: AnnotationSpan) -> IResult<AnnotationSpan, AnnotationInfo> {
    let (end_span, ty) = map(terminated(parse_basictype, tag("?")), |a| match a.tag {
        AnnotationTag::Type(ty) => ty,
        _ => unimplemented!(),
    })
    .parse(start_span)?;
    let satrt_position = Position::new(start_span.location_line(), start_span.get_column() as u32);
    let end_position = Position::new(end_span.location_line(), end_span.get_column() as u32);
    Ok((
        end_span,
        AnnotationInfo {
            tag: AnnotationTag::Type(TypeKind::Union(vec![ty, TypeKind::Nil])),
            span: Span {
                start: satrt_position,
                end: end_position,
            },
        },
    ))
}

fn parse_union(start_span: AnnotationSpan) -> IResult<AnnotationSpan, AnnotationInfo> {
    let (end_span, tys) = map(
        separated_list1(ws(tag("|")), parse_basictype),
        |ann_infos| {
            ann_infos
                .iter()
                .map(|ann| match ann.tag.clone() {
                    AnnotationTag::Type(ty) => ty,
                    _ => unimplemented!(),
                })
                .collect::<Vec<TypeKind>>()
        },
    )
    .parse(start_span)?;
    let satrt_position = Position::new(start_span.location_line(), start_span.get_column() as u32);
    let end_position = Position::new(end_span.location_line(), end_span.get_column() as u32);
    if tys.len() >= 2 {
        Ok((
            end_span,
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::Union(tys)),
                span: Span {
                    start: satrt_position,
                    end: end_position,
                },
            },
        ))
    } else {
        Err(nom::Err::Error(nom::error::Error::new(
            start_span,
            nom::error::ErrorKind::SeparatedList,
        )))
    }
}

fn parse_array(start_span: AnnotationSpan) -> IResult<AnnotationSpan, AnnotationInfo> {
    let (end_span, ty) = map(terminated(parse_basictype, tag("[]")), |ann| {
        match ann.tag {
            AnnotationTag::Type(ty) => ty,
            _ => unimplemented!(),
        }
    })
    .parse(start_span)?;
    let satrt_position = Position::new(start_span.location_line(), start_span.get_column() as u32);
    let end_position = Position::new(end_span.location_line(), end_span.get_column() as u32);
    Ok((
        end_span,
        AnnotationInfo {
            tag: AnnotationTag::Type(TypeKind::Array(Box::new(ty))),
            span: Span {
                start: satrt_position,
                end: end_position,
            },
        },
    ))
}

fn parse_tabletype(start_span: AnnotationSpan) -> IResult<AnnotationSpan, AnnotationInfo> {
    let (end_span, _) = tag("table").parse(start_span)?;
    let (end_span, (key_ty, val_ty)) = map(
        delimited(
            char('<'),
            separated_pair(parse_basictype, ws(char(',')), parse_basictype),
            char('>'),
        ),
        |(key, val)| match (key.tag, val.tag) {
            (AnnotationTag::Type(key_ty), AnnotationTag::Type(val_ty)) => (key_ty, val_ty),
            (_, _) => unimplemented!(),
        },
    )
    .parse(end_span)?;
    let satrt_position = Position::new(start_span.location_line(), start_span.get_column() as u32);
    let end_position = Position::new(end_span.location_line(), end_span.get_column() as u32);
    Ok((
        end_span,
        AnnotationInfo {
            tag: AnnotationTag::Type(TypeKind::KVTable {
                key: Box::new(key_ty),
                val: Box::new(val_ty),
            }),
            span: Span {
                start: satrt_position,
                end: end_position,
            },
        },
    ))
}

fn parse_dict(start_span: AnnotationSpan) -> IResult<AnnotationSpan, AnnotationInfo> {
    let (end_span, (key_ty, val_ty)) = map(
        delimited(
            ws(char('{')),
            separated_pair(
                delimited(char('['), parse_basictype, char(']')),
                ws(char(':')),
                parse_basictype,
            ),
            ws(char('}')),
        ),
        |(key, val)| match (key.tag, val.tag) {
            (AnnotationTag::Type(key_ty), AnnotationTag::Type(val_ty)) => (key_ty, val_ty),
            (_, _) => unimplemented!(),
        },
    )
    .parse(start_span)?;
    let satrt_position = Position::new(start_span.location_line(), start_span.get_column() as u32);
    let end_position = Position::new(end_span.location_line(), end_span.get_column() as u32);
    Ok((
        end_span,
        AnnotationInfo {
            tag: AnnotationTag::Type(TypeKind::Dict {
                key: Box::new(key_ty),
                val: Box::new(val_ty),
            }),
            span: Span {
                start: satrt_position,
                end: end_position,
            },
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
    use super::*;
    use pretty_assertions::assert_eq;
    #[test]
    fn type_annotation() {
        // sigle type
        let content = "---@type number";
        let ann_infos = parse_annotation(content);
        assert_eq!(ann_infos.len(), 1);
        assert_eq!(
            ann_infos[0],
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::Number),
                span: Span {
                    start: Position::new(1, 10),
                    end: Position::new(1, 16)
                }
            }
        );
        // multi assign
        let content = "---@type number,string";
        let ann_infos = parse_annotation(content);
        assert_eq!(ann_infos.len(), 2);
        assert_eq!(
            ann_infos[0],
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::Number),
                span: Span {
                    start: Position::new(1, 10),
                    end: Position::new(1, 16),
                }
            }
        );
        assert_eq!(
            ann_infos[1],
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::String),
                span: Span {
                    start: Position::new(1, 17),
                    end: Position::new(1, 23),
                }
            }
        );
        // optional
        let content = "---@type number?";
        let ann_infos = parse_annotation(content);
        assert_eq!(ann_infos.len(), 1);
        assert_eq!(
            ann_infos[0],
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::Union(vec![TypeKind::Number, TypeKind::Nil])),
                span: Span {
                    start: Position::new(1, 10),
                    end: Position::new(1, 17)
                }
            }
        );
        // union
        let content = "---@type number|string";
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.len(), 1);
        assert_eq!(
            ann_info[0],
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::Union(vec![TypeKind::Number, TypeKind::String])),
                span: Span {
                    start: Position::new(1, 10),
                    end: Position::new(1, 23)
                }
            }
        );
        // if no annotation, return any type
        let content = "";
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.is_empty(), true);
        // array
        let content = "---@type string[]";
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.len(), 1);
        assert_eq!(
            ann_info[0],
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::Array(Box::new(TypeKind::String))),
                span: Span {
                    start: Position::new(1, 10),
                    end: Position::new(1, 18),
                }
            }
        );
        // dictionary
        let content = "---@type { [string]: boolean }";
        let ann_info = parse_annotation(content);
        assert_eq!(ann_info.len(), 1);
        assert_eq!(
            ann_info[0],
            AnnotationInfo {
                tag: AnnotationTag::Type(TypeKind::Dict {
                    key: Box::new(TypeKind::String),
                    val: Box::new(TypeKind::Boolean(BoolLiteral::Any)),
                }),
                span: Span {
                    start: Position::new(1, 10),
                    end: Position::new(1, 31),
                }
            }
        );
        // table
        let content = "---@type table<string, number>";
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
                    end: Position::new(1, 31),
                }
            }
        );
    }
}
