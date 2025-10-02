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

pub fn collect_source_files(target: &PathBuf, config: &Config) -> Result<Vec<PathBuf>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

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
                "typua-workspace-test-{:?}-{timestamp}",
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

    #[test]
    fn expand_pattern_handles_home_and_env() {
        let temp = TestDir::new();
        let home_dir = temp.path().join("home");
        fs::create_dir_all(&home_dir).expect("create home dir");

        let original_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", &home_dir);
        }

        let with_home = expand_pattern("$HOME/project/init.lua");
        assert!(with_home.starts_with(home_dir.to_string_lossy().as_ref()));

        let with_tilde = expand_pattern("~/project/init.lua");
        assert!(with_tilde.starts_with(home_dir.to_string_lossy().as_ref()));

        match original_home {
            Some(original) => unsafe {
                std::env::set_var("HOME", original);
            },
            None => unsafe {
                std::env::remove_var("HOME");
            },
        }
    }

    #[test]
    fn collect_source_files_respects_include_patterns() {
        let temp = TestDir::new();
        let root = temp.path();
        let lua_root = root.join("main.lua");
        let mut file = File::create(&lua_root).expect("create main.lua");
        writeln!(file, "print('root')").expect("write main.lua");
        drop(file);

        let subdir = root.join("sub");
        fs::create_dir_all(&subdir).expect("create subdir");
        let lua_sub = subdir.join("module.lua");
        let mut file = File::create(&lua_sub).expect("create module.lua");
        writeln!(file, "return {{}}").expect("write module.lua");
        drop(file);

        let ignored = root.join("ignore.txt");
        let mut file = File::create(&ignored).expect("create ignore");
        writeln!(file, "should be ignored").expect("write ignore");
        drop(file);

        let mut config = Config::default();
        config.runtime.include = vec!["*.lua".to_string(), "sub/*.lua".to_string()];

        let files =
            collect_source_files(&root.to_path_buf(), &config).expect("collect source files");
        let canonical_files: Vec<PathBuf> = files
            .into_iter()
            .map(|path| path.canonicalize().unwrap())
            .collect();

        assert_eq!(canonical_files.len(), 2);
        assert!(canonical_files.contains(&lua_root.canonicalize().unwrap()));
        assert!(canonical_files.contains(&lua_sub.canonicalize().unwrap()));
    }
}
