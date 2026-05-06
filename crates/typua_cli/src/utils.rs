use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// walk recursively directory path and so return paths matching file extention
/// ```rust
/// assert_eq!(
///     walkdir_exts(".", ["lua", "toml"]),
///     Ok(vec![
///         PathBuf::from("src/init.lua"),
///         PathBuf::from("config.toml"),
///     ])
/// ```
pub fn walkdir_exts(
    dir: impl AsRef<Path>,
    exts: impl IntoIterator<Item = impl AsRef<str>>,
) -> io::Result<Vec<PathBuf>> {
    let exts: Vec<String> = exts
        .into_iter()
        .map(|ext| ext.as_ref().trim_start_matches(".").to_string())
        .collect();
    let pred = |p: &Path| {
        p.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| exts.iter().any(|allow| allow == ext))
    };
    walkdir_filtered(dir, &pred)
}

/// walk recursively directory path and so return paths what `predicate` returns true.
/// ```rust
/// assert_eq!(
///     walkdir_exts(".", ["lua", "toml"]),
///     Ok(vec![
///         PathBuf::from("src/init.lua"),
///         PathBuf::from("config.toml"),
///     ])
/// ```
fn walkdir_filtered<F>(dir: impl AsRef<Path>, predicate: &F) -> io::Result<Vec<PathBuf>>
where
    F: Fn(&Path) -> bool,
{
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let childrens = walkdir_filtered(&path, predicate)?;
            paths.extend_from_slice(&childrens);
        } else if predicate(&path) {
            paths.push(path.clone());
        }
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    #[test]
    fn test_walk_filtered() -> io::Result<()> {
        let dir = tempdir()?;
        std::fs::write(dir.path().join("README.md"), "# test")?;
        fs::create_dir_all(dir.path().join("src/mod1"))?;
        std::fs::write(dir.path().join("src/init.lua"), "return {}")?;
        std::fs::write(dir.path().join("src/config.lua"), "")?;
        std::fs::write(dir.path().join("src/mod1/init.lua"), "")?;

        fs::create_dir_all(dir.path().join("crates"))?;
        std::fs::write(dir.path().join("crates/main.rs"), "fn main() {}")?;

        let root = dir.path();
        let paths = walkdir_exts(root, ["lua"]);
        match paths {
            Ok(paths) => {
                println!("{paths:#?}");
            }
            Err(err) => {
                eprint!("Error!: {err:#?}");
            }
        }
        Ok(())
    }
}
