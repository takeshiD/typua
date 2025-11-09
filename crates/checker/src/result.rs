use typua_span::Span;
use typua_ty::{diagnostic::Diagnostic, kind::TypeKind};

#[derive(Debug, Clone, Default)]
pub struct CheckResult {
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckResult {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }
    pub fn merge(&self, other: &CheckResult) -> CheckResult {
        let mut new_diagnostics = self.diagnostics.clone();
        new_diagnostics.extend(other.diagnostics.clone());
        CheckResult {
            diagnostics: new_diagnostics,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvalType {
    pub span: Span,
    pub ty: TypeKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvalErr {
    pub span: Span,
    pub diagnostic: Diagnostic,
}
