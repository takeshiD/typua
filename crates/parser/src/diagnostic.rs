use typua_span::Span;

#[salsa::accumulator]
#[derive(Debug)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn new(span: Span, message: String) -> Self {
        Self { message, span }
    }
}
