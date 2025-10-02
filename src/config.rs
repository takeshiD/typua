use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::error::{Result, TypuaError};

const DEFAULT_CONFIG_FILENAME: &str = ".typua.toml";

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub runtime: RuntimeConfig,
    pub workspace: WorkspaceConfig,
}

impl Config {
    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        let path = dir.join(DEFAULT_CONFIG_FILENAME);
        if !path.exists() {
            return Ok(Self::default());
        }

        Self::load_from_file(path)
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let raw = fs::read_to_string(&path).map_err(|source| TypuaError::ConfigIo {
            path: path.clone(),
            source,
        })?;
        let config =
            toml::from_str(&raw).map_err(|source| TypuaError::ConfigParse { path, source })?;
        Ok(config)
    }

    pub fn config_path(dir: &Path) -> PathBuf {
        dir.join(DEFAULT_CONFIG_FILENAME)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RuntimeConfig {
    pub version: RuntimeVersion,
    pub include: Vec<String>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            version: RuntimeVersion::Luajit,
            include: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum RuntimeVersion {
    Lua51,
    Lua52,
    Lua53,
    Lua54,
    #[default]
    Luajit,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct WorkspaceConfig {
    pub library: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDir {
        path: std::path::PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let mut path = std::env::temp_dir();
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos();
            path.push(format!(
                "typua-config-test-{:?}-{timestamp}",
                std::thread::current().id()
            ));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn path(&self) -> &std::path::Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn load_from_dir_returns_default_when_missing() {
        let temp = TestDir::new();
        let config = Config::load_from_dir(temp.path()).expect("load config");
        assert!(matches!(config.runtime.version, RuntimeVersion::Luajit));
        assert!(config.runtime.include.is_empty());
        assert!(config.workspace.library.is_empty());
    }

    #[test]
    fn load_from_dir_reads_typua_toml() {
        let temp = TestDir::new();
        let config_path = temp.path().join(".typua.toml");
        let mut file = std::fs::File::create(&config_path).expect("create config file");
        writeln!(
            file,
            "[runtime]\nversion = \"lua53\"\ninclude = [\"src/*.lua\"]\n\n[workspace]\nlibrary = [\"/opt/lua\"]\n"
        )
        .expect("write config");

        // Drop file handle before reading to ensure contents are flushed on all platforms.
        drop(file);

        let config = Config::load_from_dir(temp.path()).expect("load config");
        assert!(matches!(config.runtime.version, RuntimeVersion::Lua53));
        assert_eq!(config.runtime.include, vec!["src/*.lua".to_string()]);
        assert_eq!(config.workspace.library, vec!["/opt/lua".to_string()]);
    }
}
