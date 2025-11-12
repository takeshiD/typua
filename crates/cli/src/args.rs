use clap::{Parser, Subcommand};
use std::path::PathBuf;
use typua_config::LuaVersion;

#[derive(Debug, Parser)]
#[command(author, name = "typua", about = "a typechecker for lua", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Serve(ServeCommand),
    Check(CheckCommand),
}

#[derive(Debug, Parser)]
pub struct ServeCommand {}

#[derive(Debug, Parser)]
pub struct CheckCommand {
    #[arg(default_value = ".")]
    pub path: PathBuf,
    #[arg(short = 'l', long = "lua-version", default_value = "lua51")]
    pub lua_version: LuaVersion,
}
