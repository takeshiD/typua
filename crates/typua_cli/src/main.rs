mod check;
mod server;
mod utils;

use crate::check::run_check;
use crate::server::run_server;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use typua_config::LuaVersion;

#[derive(Debug, Parser)]
#[command(name = "typua")]
#[command(about = "A lua typechecker and language server")]
#[command(version)]
struct TypuaArgs {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    // check workspace
    Check(CheckOption),
    Server(ServerOption),
}

#[derive(Debug, Args)]
struct CommonOption {
    #[arg(
        short = 'c',
        long = "config",
        default_value = "typua.toml",
        value_name = "FILE",
        help = "Configure filepath"
    )]
    config: PathBuf,
}

#[derive(Debug, Args)]
struct CheckOption {
    #[command(flatten)]
    common: CommonOption,

    #[arg(value_name = "FILES", default_value = ".")]
    /// Target files or directory
    ///
    /// Specified directory: "typua check ." run checking lua files on current working directory.
    ///
    /// Specified files: "typua check a.lua b.lua" run checking specified files only.
    paths: Vec<PathBuf>,

    #[arg(
        short = 'v',
        long = "verbose",
        default_value_t = false,
        help = "Enable verbose logging"
    )]
    verbose: bool,

    #[arg(
        long = "lua",
        value_enum,
        default_value_t = LuaVersionArg::LuaJIT,
        help = "Lua version for tpyecheck"
    )]
    version: LuaVersionArg,

    #[arg(
        long = "format",
        value_enum,
        default_value_t = OutputFormat::Full,
        value_name = "FORMAT",
        help = "Output format"
    )]
    output_format: OutputFormat,
}

#[derive(ValueEnum, Debug, Clone)]
enum OutputFormat {
    Full,
    Concise,
    Json,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            Self::Full => "full",
            Self::Concise => "concise",
            Self::Json => "json",
        };
        write!(f, "{}", s)
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, Default)]
enum LuaVersionArg {
    #[default]
    Lua51,
    LuaJIT,
}

impl From<LuaVersionArg> for LuaVersion {
    fn from(version: LuaVersionArg) -> LuaVersion {
        match version {
            LuaVersionArg::Lua51 => LuaVersion::Lua51,
            LuaVersionArg::LuaJIT => LuaVersion::LuaJIT,
        }
    }
}

#[derive(Debug, Args)]
struct ServerOption {
    #[command(flatten)]
    common: CommonOption,
}

fn main() -> std::io::Result<()> {
    let args = TypuaArgs::parse();
    println!("{:#?}", args);
    match args.commands {
        Commands::Check(opt) => run_check(opt.paths, opt.version.into()),
        Commands::Server(_opt) => {
            let root = std::env::current_dir()?;
            run_server(root)
        }
    }
    Ok(())
}
