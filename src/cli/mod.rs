use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{
    config::Config,
    error::{Result, TypuaError},
};

#[derive(Debug)]
pub enum Command {
    Check(CheckOptions),
    Lsp(LspOptions),
}

#[derive(Debug, Clone)]
pub struct CheckOptions {
    pub target: PathBuf,
    pub config: Config,
}

#[derive(Debug, Clone)]
pub struct LspOptions {
    pub root: PathBuf,
    pub config: Config,
}

#[derive(Parser, Debug)]
#[command(name = "typua", version, about = "A Lua type checker and LSP server")]
struct Cli {
    #[command(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand, Debug)]
enum Subcommands {
    /// Run the type checker over a path
    Check {
        /// Path to a file or directory containing Lua sources
        path: PathBuf,
    },
    /// Start the Typua language server
    Lsp,
}

pub fn parse() -> Result<Command> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().map_err(|source| TypuaError::CurrentDir { source })?;
    let config = Config::load_from_dir(&cwd)?;

    let command = match cli.command {
        Subcommands::Check { path } => Command::Check(CheckOptions {
            target: path,
            config,
        }),
        Subcommands::Lsp => Command::Lsp(LspOptions { root: cwd, config }),
    };

    Ok(command)
}
