use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use crate::{
    config::Config,
    error::{Result, TypuaError},
};

#[derive(Debug)]
pub enum Command {
    Check(CheckOptions),
    Lsp(LspOptions),
}

#[derive(Debug, Clone)]
pub struct CheckOptions {
    pub target: PathBuf,
    pub config: Config,
}

#[derive(Debug, Clone)]
pub struct LspOptions {
    pub root: PathBuf,
    pub config: Config,
}

#[derive(Parser, Debug)]
#[command(name = "typua", version, about = "A Lua type checker and LSP server")]
struct Cli {
    /// Path to an explicit configuration file.
    #[arg(short, long, global = true, value_name = "CONFIG")]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand, Debug)]
enum Subcommands {
    /// Run the type checker over a path
    Check {
        /// Path to a file or directory containing Lua sources
        path: PathBuf,
    },
    /// Start the Typua language server
    Lsp,
}

pub fn parse() -> Result<Command> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().map_err(|source| TypuaError::CurrentDir { source })?;
    let config = load_config(&cwd, cli.config.as_ref())?;

    let command = match cli.command {
        Subcommands::Check { path } => Command::Check(CheckOptions {
            target: path,
            config,
        }),
        Subcommands::Lsp => Command::Lsp(LspOptions { root: cwd, config }),
    };

    Ok(command)
}

fn load_config(cwd: &Path, override_path: Option<&PathBuf>) -> Result<Config> {
    if let Some(path) = override_path {
        let resolved = if path.is_absolute() {
            path.clone()
        } else {
            cwd.join(path)
        };
        Config::load_from_file(resolved)
    } else {
        Config::load_from_dir(cwd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RuntimeVersion;
    use std::fs::{self, File};
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};
    use unindent::unindent;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let mut path = std::env::temp_dir();
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos();
            path.push(format!(
                "typua-cli-test-{:?}-{timestamp}",
                std::thread::current().id()
            ));
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_config(path: &Path, contents: &str) {
        let mut file = File::create(path).expect("create config file");
        write!(file, "{contents}").expect("write config");
    }

    #[test]
    fn load_config_uses_cusutom_and_default() {
        let temp = TestDir::new();
        let relative_path = temp.path().join("custom.toml");
        let toml_source = unindent(
            r#"
            [runtime]
            version = "lua54"

            [workspace]
            library = []
        "#,
        );
        write_config(&relative_path, &toml_source);

        // Also place a default config that should be ignored.
        let default_path = temp.path().join(".typua.toml");
        let toml_source = unindent(
            r#"
            [runtime]
            version = "lua51"

            [workspace]
            library = []
        "#,
        );
        write_config(&default_path, &toml_source);

        // custom
        let config =
            load_config(temp.path(), Some(&PathBuf::from("custom.toml"))).expect("load config");
        assert_eq!(config.runtime.version, RuntimeVersion::Lua54);

        // default
        let config = load_config(temp.path(), None).expect("load default config");
        assert_eq!(config.runtime.version, RuntimeVersion::Lua51);
    }
}
