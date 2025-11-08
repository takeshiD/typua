use clap::{Parser, Subcommand};

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
pub struct CheckCommand {}
