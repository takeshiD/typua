use typua_syntax::span::Span;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    issue: Issue,
}

impl Diagnostic {
    pub fn new(kind: IssueKind, span: Span) -> Self {
        Self {
            issue: Issue { kind, span },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Issue {
    kind: IssueKind,
    span: Span,
}

#[derive(Debug, Clone)]
pub enum IssueKind {
    SyntaxError(String),
}
