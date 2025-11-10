mod args;
use crate::args::{Args, CheckCommand, Commands};
use clap::Parser;
use typua_analyzer::Analyzer;
use typua_lsp::handle_lsp_service;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Serve(_) => handle_lsp_service(),
        Commands::Check(CheckCommand { path, version }) => {
            let path = path.unwrap_or_else(|| std::env::current_dir().expect("failed get cwd"));
            let version = version.unwrap_or_default();
            let analyzer = Analyzer::new();
            analyzer.analyze(&path, &version)?
        }
    }

    Ok(())
}
