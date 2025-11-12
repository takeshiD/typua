use std::str::FromStr;

#[derive(Debug, Clone, Copy, Default)]
pub enum LuaVersion {
    #[default]
    Lua51,
    LuaJIT,
}

impl FromStr for LuaVersion {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lua51" => Ok(Self::Lua51),
            "luajit" => Ok(Self::LuaJIT),
            _ => Err(format!("invalid lua version: {}", s)),
        }
    }
}

impl From<LuaVersion> for full_moon::LuaVersion {
    fn from(version: LuaVersion) -> full_moon::LuaVersion {
        match version {
            LuaVersion::Lua51 => full_moon::LuaVersion::lua51(),
            LuaVersion::LuaJIT => full_moon::LuaVersion::luajit(),
        }
    }
}

impl std::fmt::Display for LuaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            LuaVersion::Lua51 => "lua51",
            LuaVersion::LuaJIT => "luajit",
        };
        write!(f, "{}", s)
    }
}
