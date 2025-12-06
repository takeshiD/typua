use clap::builder::styling::{AnsiColor, Styles};
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use std::path::PathBuf;
use typua_config::LuaVersion;

#[derive(Debug, Parser)]
#[command(author, name = "typua", about = "a typechecker for lua", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

impl Args {
    pub fn parse_with_color() -> Result<Self, clap::Error> {
        const STYLES: Styles = Styles::styled()
            .header(AnsiColor::Green.on_default().bold())
            .usage(AnsiColor::Green.on_default().bold())
            .literal(AnsiColor::Blue.on_default().bold())
            .placeholder(AnsiColor::Cyan.on_default());
        let cmd = Self::command().styles(STYLES);
        Self::from_arg_matches(&cmd.get_matches())
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// run language server
    Serve(ServeCommand),
    /// run instantly check directory or files
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
