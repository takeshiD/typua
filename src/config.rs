pub struct Config {
    version: LuaVersion,
}

pub enum LuaVersion {
    Lua51,
    Lua52,
    Lua53,
    Lua54,
    LuaJIT,
}

impl From<LuaVersion> for full_moon::LuaVersion {
    fn from(from: LuaVersion) -> full_moon::LuaVersion {
        match from {
            LuaVersion::Lua51 => full_moon::LuaVersion::lua51(),
            LuaVersion::Lua52 => full_moon::LuaVersion::lua52(),
            LuaVersion::Lua53 => full_moon::LuaVersion::lua53(),
            LuaVersion::Lua54 => full_moon::LuaVersion::lua54(),
            LuaVersion::LuaJIT => full_moon::LuaVersion::luajit(),
        }
    }
}
