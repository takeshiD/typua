#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub message: String,
    pub kind: DiagnosticKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticKind {
    TypeMismatch,
}
