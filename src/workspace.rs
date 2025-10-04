use std::path::{Path, PathBuf};
use std::{collections::BTreeSet, fs};

use glob::{MatchOptions, glob_with};
use tracing::{Level, event};
use walkdir::WalkDir;

use crate::{
    config::Config,
    error::{Result, TypuaError},
};

pub fn collect_source_files(target: &PathBuf, _config: &Config) -> Result<Vec<PathBuf>> {
    event!(Level::DEBUG, "get metadata {:#?}", target);
    let metadata = fs::metadata(target).map_err(|source| TypuaError::Metadata {
        path: target.to_path_buf(),
        source,
    })?;

    if metadata.is_file() {
        return Ok(vec![canonicalize_path(target)]);
    }

    if !metadata.is_dir() {
        return Err(TypuaError::UnsupportedTarget {
            path: target.to_path_buf(),
        });
    }

    let root = canonicalize_path(target);
    let mut files = BTreeSet::new();
    // for pattern in &config.runtime.path {
    //     let expanded = expand_pattern(pattern);
    //     event!(Level::DEBUG, "expanded pattern {:#?}", expanded);
    //     if expanded.trim().is_empty() {
    //         continue;
    //     }
    //     let paths = if Path::new(&expanded).is_absolute() {
    //         collect_from_pattern(&expanded)?
    //     } else {
    //         let absolute = root.join(expanded);
    //         let pattern_str = absolute.to_string_lossy().to_string();
    //         collect_from_pattern(&pattern_str)?
    //     };
    //     for p in paths.iter() {
    //         files.insert(p.clone());
    //         event!(Level::DEBUG, "add path {:#?}", p);
    //     }
    // }
    if files.is_empty() {
        collect_from_directory(&root, &mut files)?;
    }
    Ok(files.into_iter().collect())
}

fn collect_from_pattern(pattern: &str) -> Result<Vec<PathBuf>> {
    event!(Level::DEBUG, "glob path pattern '{:#?}'", pattern);
    let options = MatchOptions {
        case_sensitive: false,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };
    match glob_with(pattern, options).map_err(|source| TypuaError::IncludeGlob {
        pattern: pattern.to_string(),
        source,
    }) {
        Ok(paths) => {
            let mut collected_paths = Vec::new();
            for entry in paths {
                match entry {
                    Ok(p) => collected_paths.push(p),
                    Err(error) => {
                        event!(Level::ERROR, ?error, "failed to glob")
                    }
                }
            }
            Ok(collected_paths)
        }
        Err(e) => Err(e),
    }
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

pub fn collect_workspace_libraries(root: &Path, config: &Config) -> Result<Vec<PathBuf>> {
    let mut files = BTreeSet::new();
    let base = workspace_base(root);

    for pattern in &config.workspace.library {
        let expanded = expand_pattern(pattern);
        let trimmed = expanded.trim();
        if trimmed.is_empty() {
            continue;
        }

        if has_glob(trimmed) {
            let pattern_path = if Path::new(trimmed).is_absolute() {
                trimmed.to_string()
            } else {
                base.join(trimmed).to_string_lossy().to_string()
            };
            let paths = collect_from_pattern(&pattern_path)?;
            for path in paths {
                collect_path(&path, &mut files)?;
            }
        } else {
            let resolved = if Path::new(trimmed).is_absolute() {
                PathBuf::from(trimmed)
            } else {
                base.join(trimmed)
            };
            collect_path(&resolved, &mut files)?;
        }
    }
    Ok(files.into_iter().collect())
}

fn collect_path(path: &Path, files: &mut BTreeSet<PathBuf>) -> Result<()> {
    let metadata = fs::metadata(path).map_err(|source| TypuaError::Metadata {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.is_dir() {
        collect_from_directory(path, files)?;
    } else if metadata.is_file() {
        push_if_lua(path, files);
    }
    Ok(())
}

fn collect_from_directory(root: &Path, files: &mut BTreeSet<PathBuf>) -> Result<()> {
    for entry in WalkDir::new(root) {
        let entry = entry.map_err(|source| TypuaError::WalkDir {
            path: root.to_path_buf(),
            source,
        })?;
        if entry.file_type().is_file() {
            push_if_lua(entry.path(), files);
        }
    }
    Ok(())
}

fn push_if_lua(path: &Path, files: &mut BTreeSet<PathBuf>) {
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("lua"))
    {
        files.insert(canonicalize_path(path));
        event!(Level::DEBUG, "collected lua file {}", path.display());
    }
}

fn canonicalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn workspace_base(target: &Path) -> PathBuf {
    if target.is_dir() {
        canonicalize_path(target)
    } else {
        target
            .parent()
            .map(canonicalize_path)
            .unwrap_or_else(|| canonicalize_path(target))
    }
}

fn has_glob(pattern: &str) -> bool {
    pattern.chars().any(|c| matches!(c, '*' | '?' | '['))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::collections::BTreeSet;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::{Path, PathBuf};
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
        config.runtime.path = vec!["*.lua".to_string(), "sub/*.lua".to_string()];

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

    #[test]
    fn collect_workspace_libraries_supports_relative_paths() {
        let temp = TestDir::new();
        let root = temp.path();
        let lib_dir = root.join("lib");
        fs::create_dir_all(&lib_dir).expect("create lib dir");
        let lib_file = lib_dir.join("util.lua");
        let mut file = File::create(&lib_file).expect("create util.lua");
        writeln!(file, "return {{}}").expect("write util.lua");
        drop(file);

        let mut config = Config::default();
        config.workspace.library = vec!["lib".to_string()];

        let libs = collect_workspace_libraries(root, &config).expect("collect workspace libraries");
        let canonical_libs: Vec<PathBuf> = libs
            .into_iter()
            .map(|path| path.canonicalize().unwrap())
            .collect();

        assert_eq!(canonical_libs.len(), 1);
        assert!(canonical_libs.contains(&lib_file.canonicalize().unwrap()));
    }

    #[test]
    fn collect_workspace_libraries_supports_absolute_paths_and_globs() {
        let temp = TestDir::new();
        let root = temp.path();

        let external_dir = temp.path().join("external");
        fs::create_dir_all(&external_dir).expect("create external dir");
        let file_a = external_dir.join("a.lua");
        let mut file = File::create(&file_a).expect("create a.lua");
        writeln!(file, "return {{}}").expect("write a.lua");
        drop(file);
        let nested_dir = external_dir.join("nested");
        fs::create_dir_all(&nested_dir).expect("create nested dir");
        let file_b = nested_dir.join("b.lua");
        let mut file = File::create(&file_b).expect("create b.lua");
        writeln!(file, "return {{}}").expect("write b.lua");
        drop(file);

        let mut config = Config::default();
        config.workspace.library = vec![
            external_dir.to_string_lossy().to_string(),
            format!("{}/**/*.lua", external_dir.to_string_lossy()),
        ];

        let libs = collect_workspace_libraries(root, &config).expect("collect workspace libraries");
        let canonical_libs: BTreeSet<PathBuf> = libs
            .into_iter()
            .map(|path| path.canonicalize().unwrap())
            .collect();

        assert!(canonical_libs.contains(&file_a.canonicalize().unwrap()));
        assert!(canonical_libs.contains(&file_b.canonicalize().unwrap()));
    }
}
