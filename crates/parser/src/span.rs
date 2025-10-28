#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Position {
    line: u32,
    character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
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
