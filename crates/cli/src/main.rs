use clap::Parser;

mod args;

use crate::args::{Args, Commands};
use typua_lsp::handle_lsp_service;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Serve(_) => handle_lsp_service(),
        Commands::Check(_) => {
            println!("Check!");
        }
    }

    Ok(())
}
