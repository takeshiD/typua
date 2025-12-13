mod args;
mod lsp;

use crate::args::{Args, CheckCommand, Commands};
use crate::lsp::handle_lsp_service;
use typua_analyzer::Analyzer;
use typua_workspace::LspWorkspaceManager;

use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use anyhow::Context;
use tracing::debug;
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    let cmd = Args::parse_with_color();
    if let Ok(cmd) = cmd {
        match cmd.command {
            Commands::Serve(_) => {
                let log_name = "log.jsonl";
                let log_path = xdg::BaseDirectories::with_prefix("typua")
                    .place_cache_file(log_name)
                    .with_context(|| format!("Failed get log path: {log_name}"))?;

                let log_file =
                    if !log_path.exists() {
                        Arc::new(File::create(&log_path).with_context(|| {
                            format!("Failed to create log file: {}", log_path.display())
                        })?)
                    } else {
                        Arc::new(File::options().append(true).open(&log_path).with_context(
                            || format!("Failed to open log file: {}", log_path.display()),
                        )?)
                    };
                tracing_subscriber::fmt()
                    .with_env_filter(EnvFilter::from_default_env())
                    .with_ansi(false)
                    .with_writer(log_file)
                    .json()
                    .init();
                handle_lsp_service()
            }
            Commands::Check(CheckCommand {
                path,
                lua_version: version,
            }) => {
                tracing_subscriber::fmt()
                    .with_env_filter(EnvFilter::from_default_env())
                    .with_ansi(true)
                    .init();
                debug!("Cli options: path={}, version={}", path.display(), version);
                if path.is_dir() {
                    unimplemented!("specified path is a directory")
                } else if path.is_file() {
                    let mut f = File::open(path)?;
                    let mut content = String::new();
                    f.read_to_string(&mut content)?;
                    let workspace_manager = LspWorkspaceManager::new();
                    let analyzer = Analyzer::new();
                    let result = analyzer.analyze("", &content, &version);
                    println!("Analyze Report");
                    for d in result.diagnotics.iter() {
                        println!(
                            "Diagnostic line:{} col:{}",
                            d.span.start.line(),
                            d.span.start.character()
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
