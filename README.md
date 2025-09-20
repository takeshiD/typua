<h1 align="center">Typua</h1>

`typua` is a lua typechecker for Lua5.1, 5.2, 5.3, 5.4, LuaJIT.

`typua` type-annotation syntax is compatibled [lua-language-server](https://github.com/luals/lua-language-server).


# Features
- 🚀 Blazing Fast Typecheck
- 💾 Low memory usage
- 🖥️ Language Server Support
 

# Install

## `npm`(⚠  Planned)
```bash
npm install -g @takeshid/typua
```

## `uv`(⚠  Planned)
```bash
uv tool install typua
```

## `nix`(⚠  Planned)
```bash
nix-env --install typua
```

## `cargo`
```bash
cargo install typua
```


# Editor Integration

## nvim

### builtin-lsp
```lua
vim.lsp.enable("typua")
vim.lsp.config("typua", {
	cmd = { "typua", "lsp" },
	filetypes = { "lua" },
})
```

### lspconfig(⚠  Planned)
```lua
require("lspconfig").typua.setup({
	cmd = { "typua", "lsp" },
	filetypes = { "lua" },
}
```

# Using `typua` in CI/CD(⚠  Planned)
not yet setup


# Using `typua` as pre-commit hook(⚠  Planned)
not yet setup


# Configure
`typua`  detects `.typua.toml` in workingspace root.

on the other hand, use `--config/-c` option like `typua --config your_typua.toml`.

```toml
[runtime]
version = "luajit" # default luajit, other version lua51, lua52, lua53, lua54
include = [
    "~/.luarocks/share/lua/5.3/*.lua",
    "$HOME/.luarocks/share/lua/5.3/*.lua",
    "/usr/share/5.3/*.lua",
    "/usr/share/lua/5.3/*/init.lua",
]
```
