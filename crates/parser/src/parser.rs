use anyhow::Result;
use typua_config::LuaVersion;

use crate::ast::TypeAst;

/// entry point for parsing lua script
pub fn parse(code: &str, lua_version: LuaVersion) -> Result<TypeAst> {
    match lua_version {
        LuaVersion::Lua51 => {
            let _ = full_moon::parse_fallible(code, full_moon::LuaVersion::lua51());
            anyhow::bail!("hello")
        }
    }
}
