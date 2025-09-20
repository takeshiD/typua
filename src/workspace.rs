use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use glob::glob;
use walkdir::WalkDir;

use crate::{
    config::Config,
    error::{Result, TypuaError},
};

pub fn collect_source_files(target: &Path, config: &Config) -> Result<Vec<PathBuf>> {
    let metadata = fs::metadata(target).map_err(|source| TypuaError::Metadata {
        path: target.to_path_buf(),
        source,
    })?;

    if metadata.is_file() {
        return Ok(vec![target.to_path_buf()]);
    }

    if !metadata.is_dir() {
        return Err(TypuaError::UnsupportedTarget {
            path: target.to_path_buf(),
        });
    }

    let root = canonicalize_or_use(target);
    let mut files = BTreeSet::new();
    for pattern in &config.runtime.include {
        let expanded = expand_pattern(pattern);
        if expanded.trim().is_empty() {
            continue;
        }

        if Path::new(&expanded).is_absolute() {
            collect_from_pattern(&expanded, &mut files)?;
        } else {
            let absolute = root.join(expanded);
            let pattern_str = absolute.to_string_lossy().to_string();
            collect_from_pattern(&pattern_str, &mut files)?;
        }
    }

    if files.is_empty() {
        for entry in WalkDir::new(&root) {
            let entry = entry.map_err(|source| TypuaError::WalkDir {
                path: root.clone(),
                source,
            })?;
            if entry.file_type().is_file()
                && entry.path().extension().is_some_and(|ext| ext == "lua")
            {
                files.insert(entry.into_path());
            }
        }
    }

    Ok(files.into_iter().collect())
}

fn collect_from_pattern(pattern: &str, files: &mut BTreeSet<PathBuf>) -> Result<()> {
    let entries = glob(pattern).map_err(|source| TypuaError::IncludeGlob {
        pattern: pattern.to_string(),
        source,
    })?;

    for entry in entries {
        match entry {
            Ok(path) => {
                if path.is_file() {
                    files.insert(path);
                }
            }
            Err(error) => {
                return Err(TypuaError::IncludeGlobWalk {
                    pattern: pattern.to_string(),
                    source: error,
                });
            }
        }
    }

    Ok(())
}

fn expand_pattern(pattern: &str) -> String {
    let mut expanded = pattern.to_string();
    if let Ok(home) = std::env::var("HOME") {
        expanded = expanded.replace("$HOME", &home);
        if expanded.starts_with("~/") {
            expanded = expanded.replacen('~', &home, 1);
        } else if expanded == "~" {
            expanded = home;
        }
    }
    expanded
}

fn canonicalize_or_use(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
