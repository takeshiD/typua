use std::{fs, path::Path};

use full_moon::{self, Error as FullMoonError};

use crate::{
    cli::CheckOptions,
    diagnostics::{Diagnostic, Severity, TextPosition, TextRange},
    error::{Result, TypuaError},
    workspace,
};

#[derive(Debug, Default)]
pub struct CheckReport {
    pub files_checked: usize,
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diag| matches!(diag.severity, Severity::Error))
    }
}

pub fn run(options: &CheckOptions) -> Result<CheckReport> {
    let files = workspace::collect_source_files(&options.target, &options.config)?;

    let mut report = CheckReport {
        files_checked: files.len(),
        diagnostics: Vec::new(),
    };

    for path in &files {
        let source = read_source(path)?;
        match full_moon::parse(&source) {
            Ok(_) => {}
            Err(errors) => {
                for error in errors {
                    report.diagnostics.push(to_diagnostic(path, error));
                }
            }
        }
    }

    Ok(report)
}

fn read_source(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|source| TypuaError::SourceRead {
        path: path.to_path_buf(),
        source,
    })
}

fn to_diagnostic(path: &Path, error: FullMoonError) -> Diagnostic {
    let range = error_range(&error);
    Diagnostic::error(path.to_path_buf(), error.error_message(), range)
}

fn error_range(error: &FullMoonError) -> Option<TextRange> {
    let (start, end) = error.range();
    let start = TextPosition::from(start);
    let end = TextPosition::from(end);
    Some(TextRange { start, end })
}
