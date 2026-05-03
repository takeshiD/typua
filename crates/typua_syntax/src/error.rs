use crate::span::Span;

pub struct SyntaxError {
    span: Span,
    err: String,
}

impl SyntaxError {
    pub fn new(span: Span, err: String) -> Self {
        Self { span, err }
    }
    pub fn error(&self) -> &String {
        &self.err
    }
}

impl std::fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "SyntaxError({})", self.err)
    }
}
