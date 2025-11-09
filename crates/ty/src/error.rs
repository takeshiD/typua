use thiserror::Error;

#[derive(Debug, Error)]
pub enum TypuaError {
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),
    #[error("annotation error: {0}")]
    Annotation(#[from] AnnotationError),
    #[error("bind error: {0}")]
    Bind(#[from] BindError),
    #[error("operation error: {0}")]
    Operation(#[from] OperationError),
    #[error("failed to start tokio runtime: {source}")]
    Runtime {
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Syntax Error")]
    SyntaxError(String),
    #[error("Invalid token")]
    InvalidToken(String),
    #[error("Unexpected occured")]
    UnexpectedOccured(String),
}

#[derive(Debug, Error)]
pub enum AnnotationError {
    #[error("Invalid annotation")]
    InvalidAnnotation(String),
    #[error("Annotation syntax falied")]
    AnnotationSyntax(String),
    #[error("Unexpected occured")]
    UnexpectedOccured(String),
}

#[derive(Debug, Error)]
pub enum BindError {
    #[error("Invalid annotation")]
    InsertionFailed(String),
    #[error("Unexpected occured")]
    UnexpectedOccured(String),
}

#[derive(Debug, Error)]
pub enum OperationError {
    #[error("Add operation failed")]
    AddFailed(String),
}
