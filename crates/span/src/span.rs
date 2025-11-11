use std::hash::Hash;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct Position {
    line: u32,
    character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

impl From<full_moon::tokenizer::Token> for Span {
    fn from(token: full_moon::tokenizer::Token) -> Self {
        Self {
            start: Position::from(token.start_position()),
            end: Position::from(token.end_position()),
        }
    }
}

impl From<full_moon::tokenizer::TokenReference> for Span {
    fn from(token_ref: full_moon::tokenizer::TokenReference) -> Self {
        Self {
            start: Position::from(token_ref.start_position()),
            end: Position::from(token_ref.end_position()),
        }
    }
}

impl From<full_moon::tokenizer::Position> for Position {
    fn from(p: full_moon::tokenizer::Position) -> Self {
        Self {
            line: p.line() as u32,
            character: p.character() as u32,
        }
    }
}
