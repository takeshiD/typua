use typua_span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub message: String,
    pub kind: DiagnosticKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticKind {
    TypeMismatch,
    NotDeclaredVariable,
}
