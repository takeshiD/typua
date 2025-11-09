use std::str::FromStr;
#[derive(Debug, Clone, Copy, Default)]
pub enum LuaVersion {
    #[default]
    Lua51,
}

impl FromStr for LuaVersion {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lua51" => Ok(Self::Lua51),
            _ => Err(format!("invalid lua version: {}", s)),
        }
    }
}
