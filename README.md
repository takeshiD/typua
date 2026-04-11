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

# Usage

## Flycheck
```bash
# typecheck current directory
$ typua check

# specified config file
$ typua check -c team_typua.toml 

# check files
$ typua check main.lua src/container.lua

# select format
$ typua checi --format json
```

## Languge Server
```bash
# run language server via stdio
$ typua server

# run language server via stdio
$ typua server
```

# Editor Integration

## neovim
```lua
vim.lsp.enable("typua")
vim.lsp.config("typua", {
    cmd = { "typua", "serve" },
    filetypes = { "lua" },
    root_markers = { ".git", ".typua.toml" },
    settings = {
        typua = {
            diagnostics = {
                enable = true,
                assign_type_mismatch = true,
                param_type_mismatch  = true,
                return_type_mismatch = true,
                undefined_field      = true,
                cast_type_mismatch   = true,
            },
            completion = {
                enable = true,
                auto_require = true,
            },
            inlayhints = {
                enable = true,
                variable_types = true,
                return_types = true,
                param_types = true,
            },
            hover.enable = true,
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

### Using with `lua-ls`
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
                    "assign_type_mismatch",
                    "param_type_mismatch",
                    "return_type_mismatch",
                    "undefined_field",
                }
            }
        },
    },
}
```

# Configure
`typua` detects `.typua.toml` in workspace root.

```toml
[workspace]
ignore_dir = ["target"]
use_gitignore = true

[diagnostics]
enable = true
assign_type_mismatch = true
param_type_mismatch  = true
return_type_mismatch = true
undefined_field      = true
cast_type_mismatch   = true

[completions]
enable = true
auto_require = true

[inlayhints]
enable = true
variable_types = true
return_types   = true
param_types    = true

[typechecking]
cast_number_to_integer    = true
weak_nil_check            = false
weak_union_check          = false
check_table_shape         = false

strict_unannotated_return = true
```

