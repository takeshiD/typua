use std::fs::File;
use std::process;
use std::sync::Arc;

use typua::{
    Result, TypuaError, checker,
    cli::{self, CheckOptions, Command, LspOptions},
    lsp,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    match cli::parse()? {
        Command::Check(options) => handle_check(options),
        Command::Lsp(options) => handle_lsp(options),
    }
}

fn handle_check(options: CheckOptions) -> Result<()> {
    let report = checker::run(&options)?;

    if report.diagnostics.is_empty() {
        println!("Checked {} file(s); no issues found.", report.files_checked);
        return Ok(());
    }

    for diagnostic in &report.diagnostics {
        println!("{diagnostic}");
    }

    Err(TypuaError::TypeCheckFailed {
        diagnostics: report.diagnostics.len(),
    })
}

fn handle_lsp(options: LspOptions) -> Result<()> {
    let xdg_dir = xdg::BaseDirectories::with_prefix("typua");
    let log_path = xdg_dir
        .place_cache_file("lsp.log")
        .expect("failed to create log dir");
    let log_file = if !log_path.exists() {
        Arc::new(File::create(log_path).expect("failed to create log file"))
    } else {
        Arc::new(
            File::options()
                .append(true)
                .open(log_path)
                .expect("failed to open log file"),
        )
    };
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(log_file)
        .init();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|source| TypuaError::Runtime { source })?;

    runtime.block_on(lsp::run(options))
}
