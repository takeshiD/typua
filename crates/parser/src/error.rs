use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TypuaError {
    #[error("{source}")]
    SyntaxFalied {
        source: full_moon::Error,
    },
    #[error("annotation syntax error")]
    AnnotationSyntaxError {},
}
