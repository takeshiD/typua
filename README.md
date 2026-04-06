<h1 align="center">Typua</h1>

`typua` is a lua typechecker for Lua5.1, 5.2, 5.3, 5.4, LuaJIT.

Compatibled type-annotation syntax [lua-language-server](https://github.com/luals/lua-language-server).


# Features
- 🚀 Fast and Low memory usage
- 🖥️ Easy combination with [lua-ls](https://github.com/luals/lua-language-server)
- 🌕️ Lua5.1, 5.2, 5.3, 5.4 and LuaJIT Supported

# Install

## `cargo`
```bash
cargo install typua
```

# Editor Integration

## nvim

### builtin lspconfig
```lua
vim.lsp.enable("typua")
vim.lsp.config("typua", {
    cmd = { "typua", "serve" },
    filetypes = { "lua" },
    root_markers = { ".git", ".typua.toml" },
    settings = {
        typua = {
            workspace = {
                library = {
                    vim.env.VIMRUNTIME,                 -- vim api
                    vim.fn.stdpath("data") .. "/lazy/", -- your installed plugins
                }
            },
        },
    },
})
```

# Using with `lua-ls`
`typua` can be used in combination with `lua-ls`.

```lua
{
    cmd = { "lua-language-server" },
    filetypes = { "lua" },
    settings = {
        Lua = {
            hint = { -- use with typua
                enable = false,
            },
            diagnostics = { -- use with typua
                enable = true,
                disable = {
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

# Configure
`typua`  detects `typua.toml` in workingspace root.

```toml
[rules]
assign-type-mismatch = true
assign-type-mismatch = true

[lsp]
completions = true  # default true
inlay-hints = true  # default true
inlay-hints = true  # default true

[workspace]
ignore_dir = ["target"]
use_gitignore = true
```

