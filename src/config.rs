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
