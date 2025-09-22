use std::{fmt, path::PathBuf};

use full_moon::tokenizer::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticCode {
    AssignTypeMismatch,
    ParamTypeMismatch,
    ReturnTypeMismatch,
    UndefinedField,
    SyntaxError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPosition {
    pub line: usize,
    pub character: usize,
}

impl From<Position> for TextPosition {
    fn from(position: Position) -> Self {
        Self {
            line: position.line(),
            character: position.character(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    pub start: TextPosition,
    pub end: TextPosition,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub path: PathBuf,
    pub message: String,
    pub severity: Severity,
    pub range: Option<TextRange>,
    pub code: Option<DiagnosticCode>,
}

impl Diagnostic {
    pub fn error(
        path: PathBuf,
        message: impl Into<String>,
        range: Option<TextRange>,
        code: Option<DiagnosticCode>,
    ) -> Self {
        Self {
            path,
            message: message.into(),
            severity: Severity::Error,
            range,
            code,
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.range {
            Some(range) => write!(
                f,
                "{}:{}:{}: {}",
                self.path.display(),
                range.start.line,
                range.start.character,
                self.message
            ),
            None => write!(f, "{}: {}", self.path.display(), self.message),
        }
    }
}
