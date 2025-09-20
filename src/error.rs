use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, TypuaError>;

#[derive(Debug, Error)]
pub enum TypuaError {
    #[error("failed to read config file {path}: {source}")]
    ConfigIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {source}")]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("failed to access current working directory: {source}")]
    CurrentDir {
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read metadata for {path}: {source}")]
    Metadata {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("unsupported target path {path}")]
    UnsupportedTarget { path: PathBuf },
    #[error("failed to walk directory {path}: {source}")]
    WalkDir {
        path: PathBuf,
        #[source]
        source: walkdir::Error,
    },
    #[error("invalid include pattern '{pattern}': {source}")]
    IncludeGlob {
        pattern: String,
        #[source]
        source: glob::PatternError,
    },
    #[error("failed to expand include pattern '{pattern}': {source}")]
    IncludeGlobWalk {
        pattern: String,
        #[source]
        source: glob::GlobError,
    },
    #[error("failed to read source file {path}: {source}")]
    SourceRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("type checking failed with {diagnostics} diagnostic(s)")]
    TypeCheckFailed { diagnostics: usize },
}
