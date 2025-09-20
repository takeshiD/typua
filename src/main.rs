use std::process;

use typua::{
    Result, TypuaError, checker,
    cli::{self, CheckOptions, Command, LspOptions},
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

fn handle_lsp(_options: LspOptions) -> Result<()> {
    // TODO: Wire up the LSP server implementation
    Ok(())
}
