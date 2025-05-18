use full_moon::{
    parse_fallible,
    ast::AstResult,
};
use crate::config::LuaVersion;

pub fn parse(code: &str, lua_version: LuaVersion) -> AstResult {
    parse_fallible(code, lua_version.into())
}
