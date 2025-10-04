<h1 align="center">Typua</h1>

`typua` is a lua typechecker for Lua5.1, 5.2, 5.3, 5.4, LuaJIT.

`typua` type-annotation syntax is compatibled [lua-language-server](https://github.com/luals/lua-language-server).


# Features
- 🚀 Blazing Fast Typecheck
- 💾 Low memory usage
- 🖥️ Language Server Support
- 🌕️ Lua5.1, 5.2, 5.3, 5.4 and LuaJIT Supported

## Status
- **Type declaration**
    - [x] builtin-type(nil, number, string, boolean, function, table)
    - [ ] compound-type(union, array, tuple, dictionary, key-value table)
    - [ ] class
    - [ ] enum
    - [ ] generic function
    - [ ] generic class and method
- **Type check**
    - [ ] assign-type-mismatch
    - [ ] return-type-mismatch
    - [ ] param-type-mismatch
    - [ ] field-type-mismatch
    - [ ] table-shape-mismatch
- **Suppert Lua Version**
    - [x] Lua51
    - [ ] Lua52
    - [ ] Lua53
    - [ ] Lua54
    - [ ] LuaJIT
- **performance**
    - Reducing check time
        - [x] Step1: Every time a file is changed or opened, the AST of the file is fully analyzed.
        - [ ] Step2: Every time a file is changed or opened, Incremental analyzing only the changes.
- **LSP Support**
    - [x] Diagnostics
    - [x] Inlay hints
    - [x] Hover
    - [ ] References
    - [ ] Goto Type Defenition

# Install

## `npm`(⚠  Planned)
```bash
npm install -g @takeshid/typua
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

# Using with `lua-ls`
`typua` can be used in combination with `lua-ls`

for example nvim-lspconfig, disable `lua-ls` type-check, but leave the other diagnostics enabled.
```lua
return {
	cmd = { "lua-language-server" },
	filetypes = { "lua" },
	settings = {
		Lua = {
            diagnostics = { -- use with typua
                enable = true,
                disable = { -- disable typecheck
                    "assign-type-mismatch",
                    "param-type-mismatch",
                    "return-type-mismatch",
                    "undefined-field",
                }
            }
		},
	},
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


