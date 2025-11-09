use clap::Parser;

mod args;

use crate::args::{Args, CheckCommand, Commands};
use std::{fs::File, io::Read};
use typua_binder::Binder;
use typua_checker::typecheck;
use typua_lsp::handle_lsp_service;
use typua_parser::parse;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Serve(_) => handle_lsp_service(),
        Commands::Check(CheckCommand { path, version }) => {
            let mut f = File::open(
                path.unwrap_or_else(|| std::env::current_dir().expect("failed get cwd")),
            )?;
            let mut content = String::new();
            f.read_to_string(&mut content)?;
            let (ast, errors) = parse(&content, version.unwrap_or_default());
            let mut binder = Binder::new();
            binder.bind(&ast);
            let env = binder.get_env();
            println!("Env: {:#?}", env);
            let report = typecheck(&ast, &env);
            println!("{:#?}", report);
        }
    }

    Ok(())
}
