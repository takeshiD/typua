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

impl std::fmt::Display for LuaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            LuaVersion::Lua51 => "lua51",
        };
        write!(f, "{}", s)
    }
}
