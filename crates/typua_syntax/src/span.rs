#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ByteOffset {
    offset: u32,
}

impl From<u32> for ByteOffset {
    fn from(val: u32) -> Self {
        ByteOffset { offset: val }
    }
}

// half open section
// e.g. `Span{start=0, end=10}` equals [0, 10)
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    start: ByteOffset,
    end: ByteOffset,
}

impl Span {
    pub fn new(start: impl Into<ByteOffset>, end: impl Into<ByteOffset>) -> Self {
        Self {
            start: start.into(),
            end: end.into(),
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Span {{start:{}, end:{}}}", self.start, self.end)
    }
}

impl ByteOffset {
    pub fn new(offset: u32) -> Self {
        Self { offset }
    }
}

impl std::fmt::Display for ByteOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.offset)
    }
}

// for full_moon
impl From<full_moon::tokenizer::Token> for Span {
    fn from(token: full_moon::tokenizer::Token) -> Self {
        Self {
            start: ByteOffset::from(token.start_position()),
            end: ByteOffset::from(token.end_position()),
        }
    }
}

impl From<full_moon::tokenizer::TokenReference> for Span {
    fn from(token_ref: full_moon::tokenizer::TokenReference) -> Self {
        Self {
            start: ByteOffset::from(token_ref.start_position()),
            end: ByteOffset::from(token_ref.end_position()),
        }
    }
}

impl From<full_moon::tokenizer::Position> for ByteOffset {
    fn from(p: full_moon::tokenizer::Position) -> Self {
        Self {
            offset: p.bytes() as u32,
        }
    }
}
