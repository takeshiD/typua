use std::hash::Hash;
use tower_lsp::lsp_types::{Position as LspPosition, Range as LspRange};

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
    pub fn line(&self) -> u32 {
        self.line
    }
    pub fn character(&self) -> u32 {
        self.character
    }
}

// for full_moon
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

// for lsp-types
impl From<LspRange> for Span {
    fn from(range: LspRange) -> Self {
        Self {
            start: Position::from(range.start),
            end: Position::from(range.end),
        }
    }
}

impl From<Span> for LspRange {
    fn from(span: Span) -> Self {
        Self {
            start: LspPosition::from(span.start),
            end: LspPosition::from(span.end),
        }
    }
}

impl From<LspPosition> for Position {
    fn from(position: LspPosition) -> Self {
        Self {
            line: position.line + 1,
            character: position.character + 1,
        }
    }
}

impl From<Position> for LspPosition {
    fn from(position: Position) -> Self {
        Self {
            line: position.line - 1,
            character: position.character - 1,
        }
    }
}
