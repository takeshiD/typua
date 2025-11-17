mod backend;
use crate::backend::Backend;
use tower_lsp::{LspService, Server};

use typua_ty::error::TypuaError;

async fn run_lsp_service() {
    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

/// Entry point for lsp
pub fn handle_lsp_service() {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|source| TypuaError::Runtime { source })
    {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("Failed to start runtime: {e}");
            return;
        }
    };
    runtime.block_on(run_lsp_service())
}
