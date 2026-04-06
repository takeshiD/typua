use crate::span::Span;

pub struct SyntaxError {
    span: Span,
    err: String,
}

impl SyntaxError {
    pub fn new(span: Span, err: String) -> Self {
        Self { span, err }
    }
}
