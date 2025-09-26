pub mod cli;
pub mod config;
pub mod diagnostics;
pub mod error;
pub mod lsp;
pub mod typechecker;
pub mod typing;
pub mod workspace;

pub use error::{Result, TypuaError};
pub use typechecker::{CheckReport, CheckResult, TypeInfo, checker};
