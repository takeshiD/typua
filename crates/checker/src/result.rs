use typua_span::Span;
use typua_ty::{diagnostic::Diagnostic, kind::TypeKind};

pub struct CheckResult {
    pub diagnostics: Vec<Diagnostic>,
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

