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

impl From<Diagnostic> for tower_lsp::lsp_types::Diagnostic {
    fn from(diag: Diagnostic) -> Self {
        let range = diag.span.clone().into();
        Self {
            range,
            severity: None,
            code: None,
            code_description: None,
            source: Some("typua".to_string()),
            message: diag.message.clone(),
            related_information: None,
            tags: None,
            data: None,
        }
    }
}
