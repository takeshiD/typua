use tower_lsp::{LspService, Server};
use typua_lsp::{backend::Backend, handler::EmptyHandler};
use typua_ty::error::TypuaError;

async fn run_lsp_service() {
    let empty_handler = EmptyHandler::new();
    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(|client| Backend::new(client, empty_handler));
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
