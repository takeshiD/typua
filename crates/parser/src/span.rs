#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Span {
    start: Position,
    end: Position,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Position {
    line: u32,
    character: u32,
}
