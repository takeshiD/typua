use crate::utils::walkdir_exts;
use std::fs;
use std::io;
use std::path::PathBuf;
use typua_config::LuaVersion;
use typua_hir::LuaFile;
use typua_hir::diagnostic::Diagnostic;

pub fn run_check(targets: Vec<PathBuf>, lua_version: LuaVersion) -> io::Result<()> {
    println!("{:#?}", targets);
    let mut files = Vec::new();
    let mut paths = Vec::new();
    for p in targets {
        if !p.exists() {
            continue;
        }
        if p.is_dir() {
            let lua_paths = walkdir_exts(p, ["lua"])?;
            paths.extend_from_slice(&lua_paths);
            continue;
        }
        if p.is_file()
            && p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e == "lua")
        {
            paths.push(p)
        }
    }
    for p in paths {
        if p.exists() {
            let code = fs::read_to_string(&p)?;
            let mut file = LuaFile::new(&code, lua_version);
            file.lower();
            // file.infer();
            println!("# `{}`", p.display());
            let diagnostics: Vec<Diagnostic> = file.diagnostics().cloned().collect();
            println!("{:#?}\n", diagnostics);
            files.push(file);
        }
    }
    Ok(())
}
